# Gmail Query Analysis

## Scope

The app's baseline criteria are Gmail INBOX emails that have both PDF and JSON
attachments. The analysis below used those criteria for May and February 2026,
then downloaded each JSON attachment and classified the DTE by
`identificacion.tipoDte`.

Classification rule:

- `tipoDte = "03"`: credito fiscal
- `tipoDte = "01"`: consumidor final

The downloaded JSON files were stored under `/private/tmp` only and should not
be committed.

## May 2026

Date range:

```text
after:2026/05/01 before:2026/06/01
```

Results:

- PDF messages: 22
- JSON messages: 13
- Both PDF and JSON: 13
- Parsed JSONs: 13
- Credito fiscal, `tipoDte = "03"`: 4
- Consumidor final, `tipoDte = "01"`: 9

Temporary JSON directory:

```text
/private/tmp/dte-email-analysis-202605.dN3y2V
```

The 4 credito fiscal emails all matched:

- From: `noreply@infile.sv`
- Subject: `Documento electronico`
- Text included: `Comprobante de credito`
- Control number started with `DTE-03`
- Issuer: `ECSA OPERADORA EL SALVADOR...`

These May filters both returned exactly the 4 credito fiscal emails:

```text
after:2026/05/01 before:2026/06/01 filename:pdf filename:json "Comprobante de crédito"
after:2026/05/01 before:2026/06/01 filename:pdf filename:json DTE-03
```

The combined attachment query also returned the same 13 candidates as the
current two-search intersection approach:

```text
after:2026/05/01 before:2026/06/01 filename:pdf filename:json
```

## February 2026

Date range:

```text
after:2026/02/01 before:2026/03/01
```

Results:

- PDF messages: 28
- JSON messages: 21
- Both PDF and JSON: 21
- Parsed JSONs: 21
- Credito fiscal, `tipoDte = "03"`: 5
- Consumidor final, `tipoDte = "01"`: 16

Temporary JSON directory:

```text
/private/tmp/dte-email-analysis-202602.lk9D3q
```

The 5 credito fiscal emails came from:

- `ECSA OPERADORA EL SALVADOR...`: 3
- `UNILLANTAS S.A DE C.V`: 1
- `RAMIREZ VENTURA S.A. DE C.V.`: 1

This confirms that the May pattern was too sender-specific. February had
credito fiscal emails from non-gas-station senders, and not all visible snippets
were clearly phrased as `Comprobante de credito`.

February filter tests:

- `filename:pdf filename:json "Comprobante de crédito"` returned 5, all CCF.
- `filename:pdf filename:json DTE-03` returned 5, all CCF.
- `filename:pdf filename:json 056282773` returned 5, all CCF.
- `filename:pdf filename:json 3479152` returned 5, all CCF.
- `filename:pdf filename:json "Comprobante de credito"` returned 0.
- `filename:pdf filename:json tipoDte 03` returned 8, not clean.
- `filename:pdf filename:json -DTE-01` returned 5, all CCF.

Gmail likely matched `Comprobante de crédito`, `DTE-03`, NIT, and NRC from
indexed attachment content, not only visible email body text.

## Recommendation

Use `DTE-03` as the Gmail prefilter for credito fiscal candidates:

```text
after:{start} before:{end_exclusive} filename:pdf filename:json DTE-03
```

Reasons:

- It worked for both May and February.
- It maps directly to the DTE document type/control number for credito fiscal.
- It avoids accent sensitivity in `credito` vs `crédito`.
- It is less dependent on each sender's email wording.

Still treat Gmail search as a prefilter only. The app should continue to parse
the downloaded JSON and confirm:

```text
identificacion.tipoDte == "03"
```

This JSON check remains the source of truth.

## App Implications

1. Replace the current two-query PDF/JSON intersection with one Gmail query:

```text
after:{start} before:{end_exclusive} filename:pdf filename:json
```

2. Add an optional credito-fiscal prefilter:

```text
after:{start} before:{end_exclusive} filename:pdf filename:json DTE-03
```

3. Keep JSON validation after download to avoid relying completely on Gmail's
attachment indexing behavior.
