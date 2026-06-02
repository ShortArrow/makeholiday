# `text/*` — TEXT-value encoding hygiene

Built on the TEXT escape handling from [ADR-019 Step
2](../../design/019-parser-implementation.md). These rules flag
producers that forgot to escape `,` / `;` in TEXT values, escaped
them twice, or emitted a UTF-8 BOM.

Four rules ship in `icslint` v0.2.0; one
(`text/non-utf8-bytes`) is deferred until the library API gains a
`&[u8]`-accepting entry point.

---

## `text/bom` — info

Source begins with a UTF-8 byte-order mark (`U+FEFF`). Outlook is
the usual producer. Consumers are split: most tolerate the BOM, but
scripting pipelines that read the file via `head` / `grep` see a
stray prefix character on line 1.

**Trigger**

```ics
\u{FEFF}BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//x//y
...
```

**Fix**: strip the BOM (e.g. `sed -i '1s/^\xEF\xBB\xBF//' file.ics`)
or accept the warning if downstream consumers tolerate it.

---

## `text/double-escape` — warning

`SUMMARY` raw value contains a literal `\\,` or `\\;` substring.
This is the canonical signature of a producer that ran the TEXT
escape pass twice — first turning `,` into `\,`, then turning the
backslash into `\\`. The consumer renders a stray backslash in the
title.

**Trigger**

```ics
SUMMARY:Lunch\\, dinner
```

**Fix**: drop one level of escaping in the producer pipeline so
the final wire value is `Lunch\, dinner` (one backslash).

---

## `text/unescaped-comma-in-summary` — warning

`SUMMARY` raw value contains a `,` not preceded by an odd number of
backslashes. RFC 5545 §3.3.11 requires literal commas inside TEXT
values to be backslash-escaped (`\,`). Strict consumers silently
truncate the title at the first comma.

**Trigger**

```ics
SUMMARY:Lunch, dinner, snack
```

**Fix**: escape commas as `\,`:

```ics
SUMMARY:Lunch\, dinner\, snack
```

---

## `text/unescaped-semicolon-in-summary` — warning

`SUMMARY` raw value contains a `;` not preceded by an odd number of
backslashes. RFC 5545 §3.3.11 requires literal semicolons inside
TEXT values to be backslash-escaped (`\;`). Same silent-truncation
failure mode as commas.

**Trigger**

```ics
SUMMARY:Q1; Q2
```

**Fix**: escape semicolons as `\;`:

```ics
SUMMARY:Q1\; Q2
```

---

## `text/non-utf8-bytes` — *deferred*

> Source bytes are not valid UTF-8.

Deferred in v0.2.0. The `icslint::lint(source: &str)` entry point
accepts validated UTF-8 by construction; surfacing a non-UTF-8 input
as a diagnostic instead of an exit-3 internal error requires a
`lint_bytes(&[u8])` API variant. Tracked as a v0.2.x point-release
item.

In the meantime, non-UTF-8 inputs produce a `cannot read <path>`
message on stderr and exit code 3, which still flags the file as
unprocessable in a CI pipeline.
