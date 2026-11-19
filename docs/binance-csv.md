# Binance CSV exports → TradeLedger

> **Status: UNVERIFIED.** Column names and value formats below follow Binance's
> commonly seen export formats. Binance changes them from time to time, so they
> must be checked against a real export header before the importers are
> finished. The importers (T4.2/T4.3) match columns **by header name** and
> accept the aliases listed here, so a renamed column is a one-line fix.

All fixture files in `test-data/` contain **fake data only**.

## Where the files come from

| File                         | Binance menu (desktop)                                          | Fixture                            |
| ---------------------------- | --------------------------------------------------------------- | ---------------------------------- |
| Spot trade history           | Orders → Spot Order → Trade History → Export                    | `binance-spot-trades.csv`          |
| USDⓈ-M futures trade history | Orders → Futures Orders → USDⓈ-M → Trade History → Export       | `binance-futures-trades.csv`       |
| USDⓈ-M transaction history   | Orders → Futures Orders → USDⓈ-M → Transaction History → Export | `binance-futures-transactions.csv` |

Binance sometimes offers only `.xlsx`; save it as CSV first (Excel: Save As → CSV UTF-8).

## General rules

| Topic      | Rule                                                                                                                            |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Time       | `Date(UTC)` = `YYYY-MM-DD HH:MM:SS` in **UTC** → stored as ms since epoch.                                                      |
| Numbers    | Plain decimals with `.` (no thousands separators). Parsed straight into `rust_decimal` from the text — never through `f64`.     |
| Side       | `BUY` / `SELL` (case-insensitive) → `buy` / `sell`.                                                                             |
| Symbol     | Upper-cased, e.g. `BTCUSDT`.                                                                                                    |
| Header     | Matched by name, ignoring case, spaces and a leading UTF-8 BOM. Column order doesn't matter. Unknown extra columns are ignored. |
| Empty file | Header only → 0 rows, not an error.                                                                                             |
| Bad row    | Reported with its line number; the import is shown as a preview first, so nothing is saved until the user confirms (T4.5).      |

## 1. Spot trade history

Header: `Date(UTC),Pair,Side,Price,Executed,Amount,Fee`

| Column      | Example               | → `Fill` field      | Notes                                                          |
| ----------- | --------------------- | ------------------- | -------------------------------------------------------------- |
| `Date(UTC)` | `2026-01-05 09:15:02` | `time`              | UTC                                                            |
| `Pair`      | `ETHUSDT`             | `symbol`            |                                                                |
| `Side`      | `BUY`                 | `side`              |                                                                |
| `Price`     | `2000.25`             | `price`             | in quote asset                                                 |
| `Executed`  | `0.30000000ETH`       | `qty`               | **number + base asset glued together** → split at first letter |
| `Amount`    | `600.07500000USDT`    | (check only)        | price × qty in quote asset; used to sanity-check the row       |
| `Fee`       | `0.00030000ETH`       | `fee` + `fee_asset` | number + asset glued; asset may be base, quote or `BNB`        |

Fixed values: `exchange = "binance"`, `market = spot`, `realized_pnl = None`.

Known older variant (same meaning, separate fee asset column):
`Date(UTC),Market,Type,Price,Amount,Total,Fee,Fee Coin` → aliases `Market`=Pair, `Type`=Side, `Amount`=quantity, `Total`=quote amount, `Fee Coin`=fee asset.

### Spot fee rule (important for grouping)

When the fee is paid in the **base asset** (e.g. ETH fee on an ETH buy), you
receive less than `Executed`. Buying 0.5 ETH with a 0.0005 ETH fee leaves
0.4995 ETH, and that is all you can sell. The grouping engine therefore
treats a base-asset fee on a buy as reducing the position, otherwise the
position would never return to exactly zero. Fees in quote asset or BNB don't
change the position.

## 2. USDⓈ-M futures trade history

Header: `Date(UTC),Symbol,Side,Price,Quantity,Amount,Fee,Fee Coin,Realized Profit,Quote Asset`

| Column            | Example               | → `Fill` field | Notes                                             |
| ----------------- | --------------------- | -------------- | ------------------------------------------------- |
| `Date(UTC)`       | `2026-02-02 10:00:01` | `time`         | UTC                                               |
| `Symbol`          | `BTCUSDT`             | `symbol`       |                                                   |
| `Side`            | `BUY`                 | `side`         |                                                   |
| `Price`           | `61000.0`             | `price`        |                                                   |
| `Quantity`        | `0.006`               | `qty`          | contracts = base asset for USDⓈ-M                 |
| `Amount`          | `366.0`               | (check only)   | price × qty                                       |
| `Fee`             | `0.14640000`          | `fee`          |                                                   |
| `Fee Coin`        | `USDT`                | `fee_asset`    | can be `BNB` if BNB fee discount is on            |
| `Realized Profit` | `8.50500000`          | `realized_pnl` | 0 on opening fills; Binance's own PnL calculation |
| `Quote Asset`     | `USDT`                | (ignored)      | always USDT/USDC for USDⓈ-M                       |

Fixed values: `exchange = "binance"`, `market = usdm`.

Accepted variations:

- No `Fee Coin` column → the fee is read as number + asset glued together (`0.1464USDT`).
- No `Realized Profit` column → `realized_pnl = None` (the file is then detected only if the importer is picked by hand, since that column is what tells futures apart from spot).
- Negative fees are allowed: maker rebates on some fee tiers are paid to you.
- Aliases: `Qty` = Quantity, `Commission` = Fee, `Commission Asset` = Fee Coin, `Realized PnL` = Realized Profit.

## 3. USDⓈ-M transaction history (funding)

Header: `Date(UTC),Type,Amount,Asset,Symbol`

Only rows with `Type = FUNDING_FEE` are imported → `FundingEvent`. Other
types (`COMMISSION`, `REALIZED_PNL`, `TRANSFER`, …) are skipped: commission
and realized PnL already come from the trade history, so importing them again
would double-count.

| Column      | Example               | → `FundingEvent` field | Notes                                      |
| ----------- | --------------------- | ---------------------- | ------------------------------------------ |
| `Date(UTC)` | `2026-02-02 16:00:00` | `time`                 | funding happens at 00:00, 08:00, 16:00 UTC |
| `Amount`    | `-0.03050000`         | `amount`               | **Binance sign: negative = paid**          |
| `Asset`     | `USDT`                | (ignored)              |                                            |
| `Symbol`    | `BTCUSDT`             | `symbol`               |                                            |

Sign conversion: `Trade.funding` means "cost paid" (positive = paid), so a
trade's funding = **−(sum of its FundingEvent amounts)**.

## Duplicate protection (`dedupe_hash`)

These exports have **no trade ID**, so a row is identified by its content:

```
dedupe_hash = SHA-256( file_kind | normalized row values | occurrence )
```

- `file_kind` = `binance-spot` / `binance-usdm` / `binance-usdm-tx`.
- normalized values = the parsed fields (time ms, symbol, side, price, qty, fee, …), so formatting differences (`61000.0` vs `61000.00`) don't create a "new" row.
- `occurrence` = 1 for the first identical row in a file, 2 for the second, … Two genuinely separate fills can be identical (same second, price and qty, see the SOL rows in the spot fixture); the counter keeps both, and re-importing the same file produces the same counters, so nothing is added twice.

## Fixture scenarios and expected trades

Used by the importer tests (T4.2/T4.3) and the grouping tests (T4.4).

### Spot (`binance-spot-trades.csv`, 8 rows)

| #   | Symbol  | Fills                                       | Expected trade                                                                 |
| --- | ------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| S1  | ETHUSDT | 2 buys (partial fills, same second), 1 sell | 1 closed **long**; fee in ETH on buys → position 0.4995, sell 0.4995 closes it |
| S2  | SOLUSDT | 3 buys (**two identical rows**), 1 sell     | 1 closed long, loss; fees in BNB; both identical rows kept                     |
| S3  | BTCUSDT | 1 buy                                       | 1 **open** long                                                                |

### Futures (`binance-futures-trades.csv`, 8 rows)

| #   | Symbol  | Fills                                           | Expected trade(s)                                                                                   |
| --- | ------- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| F1  | BTCUSDT | buy 0.006 + buy 0.004 (same second), sell 0.010 | 1 closed long, realized +8.505, fees 0.491402                                                       |
| F2  | ETHUSDT | sell 0.5, **buy 0.8** (flip), sell 0.3          | **2 trades**: short 0.5 (realized +20), then long 0.3 (realized +6). The 0.8 buy is split 0.5 / 0.3 |
| F3  | SOLUSDT | sell 10, buy 10                                 | 1 closed short, realized −22                                                                        |

### Funding (`binance-futures-transactions.csv`, 9 rows → 6 imported)

| Symbol  | FUNDING_FEE rows (amount)  | Belongs to | Trade funding (cost) |
| ------- | -------------------------- | ---------- | -------------------- |
| BTCUSDT | −0.0305, −0.02745, +0.0061 | F1         | +0.05185 (paid)      |
| ETHUSDT | +0.1025 (during short)     | F2 short   | −0.1025 (received)   |
| ETHUSDT | −0.03015 (during long)     | F2 long    | +0.03015 (paid)      |
| SOLUSDT | +0.095                     | F3         | −0.095 (received)    |

Skipped rows: 1 `TRANSFER`, 1 `COMMISSION`, 1 `REALIZED_PNL`.

### Expected trade values (checked by `backend/src/features/grouping/tests.rs`)

| #        | Dir   | Status | Qty   | Avg entry | Avg exit | Gross     | Fees                           | Funding | Net      |
| -------- | ----- | ------ | ----- | --------- | -------- | --------- | ------------------------------ | ------- | -------- |
| S1       | long  | closed | 0.5   | 2000.35   | 2100     | 49.775175 | 2.049125                       | 0       | 47.72605 |
| S2       | long  | closed | 2.5   | 95.12     | 92.4     | −6.8      | 0 (+0.0006125 BNB unconverted) | 0       | −6.8     |
| S3       | long  | open   | 0.001 | 60500     | —        | 0         | 0.0605                         | 0       | −0.0605  |
| F1       | long  | closed | 0.01  | 61000     | 61850.5  | 8.505     | 0.491402                       | 0.05185 | 7.961748 |
| F2 short | short | closed | 0.5   | 2050      | 2010     | 20        | 0.812                          | −0.1025 | 19.2905  |
| F2 long  | long  | closed | 0.3   | 2010      | 2030     | 6         | 0.4848                         | 0.03015 | 5.48505  |
| F3       | short | closed | 10    | 95        | 97.2     | −22       | 0.7688                         | −0.095  | −22.6738 |

Funding column = cost (positive = paid). Net = gross − fees − funding.
