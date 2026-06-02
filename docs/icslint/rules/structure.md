# `structure/*` — physical-layer well-formedness

Rules that walk the raw source bytes (or pre-unfold logical lines)
to catch structural violations the typed parser already tolerates:
overlong physical lines, bare-LF line endings, mismatched
`BEGIN`/`END` markers, and entirely empty calendars.

All four rules ship in `icslint` v0.2.0.

---

## `structure/unfolded-long-line` — warning

A physical line exceeds 75 octets. RFC 5545 §3.1 caps physical
lines at 75 octets and requires longer content to be folded;
unfolded long lines break naive line-by-line readers and some
downstream pipelines.

**Trigger** (PRODID with a 90-octet body)

```ics
PRODID:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

**Fix**: fold per RFC 5545 §3.1 — break the line at any character
boundary and prefix the continuation with one space or tab:

```ics
PRODID:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
 xxxxxxxxxxxxxxxxxxxx
```

---

## `structure/crlf-violation` — warning

Source uses bare `\n` line endings instead of `\r\n`. RFC 5545 §3.1
requires CRLF. Most consumers tolerate LF, but strict serializers
silently lose data on a round trip through a tool that re-emits
canonical CRLF.

Fires once at the first offending line — re-emitting on every LF
would only flood the output and tell the author nothing new.

**Fix**: convert line endings to CRLF (e.g.
`unix2dos calendar.ics` on Linux).

---

## `structure/orphan-end` — error

`END:NAME` has no matching `BEGIN:NAME` at the same nesting level —
either a stray line or mismatched nesting. Both break component
reconstruction on the consumer side.

**Trigger** (extra `END:VEVENT` at top level)

```ics
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//x//y
END:VEVENT
END:VCALENDAR
```

**Fix**: remove the orphan `END:` or add the corresponding
`BEGIN:` so the block reconstructs.

---

## `structure/empty-calendar` — info

`VCALENDAR` contains zero components — no `VEVENT`, no nested
`VTIMEZONE`, no `VTODO`, no `VJOURNAL`. Valid per RFC 5545 but
almost always an accidental empty file or a stripped export.

**Trigger**

```ics
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//x//y
END:VCALENDAR
```

**Fix**: add the intended components, or accept the info diagnostic
if a deliberately empty calendar is the right output (rare).
