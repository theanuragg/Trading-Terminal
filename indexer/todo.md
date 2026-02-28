Use Jetstreamer + Old Faithful → filter SPL Token → store in Postgres → expose API.

You want a GMGM/axiom-style SPL token indexer specifically, and you want the steps tied to docs. I’ll lay this out as a linear plan: for each step I’ll say “read this” then “build this”.

Step 0 – Pick infra and target

Use your cloud credits for one fat VM (doesn’t matter which cloud):

- 8+ vCPU, 32–64 GB RAM, fast SSD/NVMe.

- Install Ubuntu/Debian.

Target: index SPL Token Program (‎⁠Tokenkeg...⁠) plus a whitelist of mints you care about (memes, OF tokens, etc).

Step 1 – Learn Jetstreamer’s model from the repo

Docs / resources:

- Jetstreamer repo: Jetstreamer GitHub (https://github.com/anza-xyz/jetstreamer)
(scroll README, examples, config)

What to learn from that:

- Jetstreamer can stream historical + live Solana data via Firehose‑style endpoints.

- How it connects to Old Faithful (full-chain archive) vs a local geyser.

- The data types you get: blocks, transactions, instructions, accounts.

What to do:

1. Clone the repo on your VM.

2. Build the examples (‎⁠cargo build --release⁠).

3. Run one of the sample streamers against Old Faithful just to print blocks/txs.

Goal: you can see a stream of real Solana blocks/tx JSON on your terminal.

Step 2 – Understand SPL token transfers at the tx level

You already know Anchor, but for indexing you need wire‑level patterns.

Docs / resources:

- Solana SPL Token program reference (Google: “SPL Token Program Solana docs” – use any official Solana/SPL page you like).

- Any tx explorer showing raw SPL transfers (Solscan, SolanaFM, etc.) for a known token.

What to learn:

- Instruction layout for:

 ▫ ‎⁠Transfer⁠, ‎⁠TransferChecked⁠, ‎⁠MintTo⁠, ‎⁠Burn⁠.

- Where the mint appears in the accounts list.

- How to distinguish:

 ▫ native SOL transfers vs SPL Token transfers,

 ▫ token account vs owner wallet.

What to do:

1. Pick one SPL token (e.g., USDC or one OF token).

2. Manually inspect 10–20 example txs and write down:

 ▫ Which program id = SPL Token.

 ▫ Which account index is the mint.

 ▫ How to compute amount (decimals).

This is what your indexer will codify in Rust.

Step 3 – Design your SPL index schema (Postgres)

Docs / resources:

- ‎⁠sqlx⁠ docs: sqlx GitHub (https://github.com/launchbadge/sqlx)

- (Optional but useful) Helius indexing patterns: search “Helius how to index solana data”.

What to design:

Minimal tables:

- ‎⁠mints(mint_pubkey, symbol, decimals, first_seen_slot, …)⁠

- ‎⁠token_transfers( id SERIAL, signature TEXT, slot BIGINT, block_time TIMESTAMPTZ, mint_pubkey TEXT, source_owner TEXT, dest_owner TEXT, source_ata TEXT, dest_ata TEXT, amount NUMERIC, tx_index INT, ix_index INT, UNIQUE(signature, ix_index) )⁠

You can add balances later; start with transfers.

What to do:

1. Launch Postgres on the same box (or managed Postgres with credits).

2. Write DDL + set up ‎⁠sqlx⁠ migrations.

3. Verify you can insert a fake ‎⁠token_transfers⁠ row from a tiny Rust script.

Step 4 – Write a Rust Firehose client on top of Jetstreamer

Here you glue Jetstreamer → Rust → DB.

Docs / resources:

- Jetstreamer crate docs (linked from Jetstreamer GitHub (https://github.com/anza-xyz/jetstreamer)): look for ‎⁠jetstreamer_firehose⁠.

- Tokio tutorial: Tokio tutorial (https://tokio.rs/tokio/tutorial)

- ‎⁠sqlx⁠ as above.

What to implement:

1. A Rust bin that:

 ▫ Connects to Jetstreamer’s Firehose endpoint (from the examples).

 ▫ Subscribes to block/transaction stream.

 ▫ For each block:

 ⁃ Iterate txs → instructions → find those with ‎⁠program_id == SPL Token program id⁠.

 ⁃ For each matching instruction, parse it into a ‎⁠TokenTransfer⁠ struct.

2. A writer task:

 ▫ Receives ‎⁠TokenTransfer⁠ via ‎⁠mpsc⁠ channel.

 ▫ Inserts batches into Postgres using ‎⁠sqlx::query!⁠.

 ▫ Enforces uniqueness via the ‎⁠UNIQUE(signature, ix_index)⁠ constraint.

3. Checkpointing:

 ▫ Keep a ‎⁠last_processed_slot⁠ table.

 ▫ After each block batch, upsert that slot.

 ▫ On restart, ask Jetstreamer for ‎⁠from_slot = last_processed_slot + 1⁠.

At this point, you have a basic GMGM-style firehose → DB pipeline, just for SPL transfers.

Step 5 – Backfill full SPL history via Old Faithful

To reach “axiom‑level” coverage, you need from genesis → now.

Docs / resources:

- Jetstreamer README sections on Old Faithful & historical streaming: Jetstreamer GitHub (https://github.com/anza-xyz/jetstreamer).

What to do:

1. Run Jetstreamer against Old Faithful in “historical catchup” mode.

2. Configure your client to:

 ▫ Start at slot 0 (or some early slot) and stream forward.

 ▫ Keep inserting into ‎⁠token_transfers⁠.

3. Let this run until you hit current tip.

4. Once caught up, switch the client into live tail mode (no gap between historical and live).

You now have an almost full-chain SPL transfer index in your DB, filtered to whatever set of mints you want.

Step 6 – Add SPL mint & balance views (gmgm-style UX)

GMGM / Axiom‑type tools care about token‑centric views.

Docs / resources:

- Same SPL docs + your own schema.

- Any block explorer’s “token holder” view as inspiration.

What to add:

1. ‎⁠balances(wallet, mint, amount)⁠ materialized view or table, updated incrementally:

 ▫ For each transfer:

 ⁃ ‎⁠source_balance -= amount⁠

 ⁃ ‎⁠dest_balance += amount⁠

2. Simple Rust API (Axum) on top:

 ▫ ‎⁠GET /token/:mint/transfers?limit=100⁠

 ▫ ‎⁠GET /token/:mint/holders?limit=100⁠

 ▫ ‎⁠GET /wallet/:owner/portfolio⁠

This is what your CLI terminal or TUI will consume.

Step 7 – Make it “axiom level”: robustness + latency

Now you polish it into something you can honestly call gmgm/axiom tier:

- Robustness

 ▫ Backoff + retry on Jetstreamer disconnect.

 ▫ Slot gap detection: if you see ‎⁠slot > last_slot + 1⁠, trigger a repair backfill.

 ▫ Metrics: processed tx/s, lag (tip_slot − last_processed_slot), DB write errors.

- Latency

 ▫ Keep parse + write path minimal (no heavy joins in the hot path).

 ▫ Do heavy analytics in background workers or separate processes.

 ▫ If needed, separate write DB (Postgres) and read DB (maybe later ClickHouse).

What to actually read / follow in order

Given everything above, a minimal reading order tied to your ask:

1. Jetstreamer README + examples – Jetstreamer GitHub (https://github.com/anza-xyz/jetstreamer)
Understand Firehose endpoints, Old Faithful, configs.

2. Tokio tutorial (only the mini-service bits) – Tokio tutorial (https://tokio.rs/tokio/tutorial)
For tasks, channels, and graceful shutdown.

3. sqlx README – sqlx GitHub (https://github.com/launchbadge/sqlx)
Just enough to do pooled connections + batched inserts.

4. SPL Token Program docs (official Solana/SPL page you prefer)
For instruction formats.

Then you implement Steps 3–6 directly in Rust.

If you stick to only those docs and that exact sequence, you’ll end up with a Jetstreamer‑powered, SPL‑focused indexer that’s structurally in the same class as what GMGM/Axiom are running, just tailored to your tokens and your trading terminal.