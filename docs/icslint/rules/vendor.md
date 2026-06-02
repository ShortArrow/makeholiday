# `vendor/*` — vendor extension hygiene

Built on the prefix-based vendor profile pre-reservation introduced
in [ADR-001](../../design/001-vendor-extension-typing.md). These
rules surface portability trade-offs of vendor-specific `X-`
properties — known vendors are flagged at `info` severity (so the
user is aware, not punished), unknown `X-` prefixes are flagged at
`warning` because they are usually typos or undocumented private
extensions.

Four rules ship in `icslint` v0.2.0; one (`vendor/typed-clash`) is
deferred until the vendor profiles grow more typed cross-vendor
slots.

---

## `vendor/microsoft-only` — info

Property uses the `X-MICROSOFT-*` prefix. Recognized by Outlook /
Exchange / Microsoft 365 but inconsistently honored by other clients.

**Trigger**

```ics
BEGIN:VEVENT
UID:e1
DTSTAMP:20260101T000000Z
DTSTART;VALUE=DATE:20260429
DTEND;VALUE=DATE:20260430
SUMMARY:s
X-MICROSOFT-CDO-BUSYSTATUS:OOF
END:VEVENT
```

**Fix**: portable equivalents where they exist — the example above
should be `TRANSP:OPAQUE` (or `TRANSP:TRANSPARENT`) for cross-client
support. Keep the Microsoft extension if the audience is Outlook-only.

---

## `vendor/google-only` — info

Property uses an `X-GOOGLE-*` or `X-GOOGLE-CALENDAR-*` prefix.
Google Calendar honors these but they rarely round-trip through
other clients.

**Trigger**

```ics
BEGIN:VEVENT
UID:e1
DTSTAMP:20260101T000000Z
DTSTART;VALUE=DATE:20260429
DTEND;VALUE=DATE:20260430
SUMMARY:s
X-GOOGLE-CONFERENCEPROPERTIES:foo
END:VEVENT
```

**Fix**: drop the extension or check whether the same information
fits in a portable property (`LOCATION` for venue, `URL` for joinable
links).

---

## `vendor/icloud-only` — info

Property uses an `X-APPLE-*` or `X-CALENDARSERVER-*` prefix. iCloud
and CalendarServer honor these but other clients usually ignore them.

**Trigger**

```ics
BEGIN:VEVENT
UID:e1
DTSTAMP:20260101T000000Z
DTSTART;VALUE=DATE:20260429
DTEND;VALUE=DATE:20260430
SUMMARY:s
X-APPLE-CALENDAR-COLOR:#FF0000
END:VEVENT
```

**Fix**: drop the extension if the audience is mixed-vendor. Color
is iCloud-specific; cross-client palettes do not yet exist in the RFC.

---

## `vendor/unrecognized-x` — warning

`X-*` property name does not match any known vendor profile.
Either a deliberate private extension or a typo of a known vendor
prefix (`X-MICROSFT-` instead of `X-MICROSOFT-`, for example).

**Trigger**

```ics
BEGIN:VEVENT
UID:e1
DTSTAMP:20260101T000000Z
DTSTART;VALUE=DATE:20260429
DTEND;VALUE=DATE:20260430
SUMMARY:s
X-CUSTOM-COLOR:blue
END:VEVENT
```

**Fix**: confirm the prefix is intentional. If it is a typo of a
known vendor, fix the spelling so it routes through the proper
vendor profile bundle.

---

## `vendor/typed-clash` — *deferred*

> Same logical concept is set via two different vendor extensions
> (e.g., busy status via Microsoft and iCloud bundles
> simultaneously).

Deferred in v0.2.0. The only typed cross-vendor slot today is
`microsoft.busystatus`; until more vendor profiles grow typed
fields covering the same logical concept, there is no clash to
detect. Will reactivate when ADR-001 typed slots expand in the
google / icloud profiles.
