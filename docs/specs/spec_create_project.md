## Task: `tzbucket` — DST-sichere Zeit-Buckets (Rust CLI + Core Library)

### Ziel

Baue ein kleines Tool **`tzbucket`**, das **Zeitstempel deterministisch** in **kalenderbasierte Buckets** (Tag/Woche/Monat) einordnet — **in einer IANA-Zeitzone** (z.B. `Europe/Berlin`) — und dabei DST-Edgecases explizit und testbar behandelt.

**Warum?** Kalender-Buckets sind in Analytics/ETL/Produktsystemen überall, DST macht „einfach day grouping“ oft falsch.

---

## MVP-Scope (v0.1)

### 1) Subcommands

#### A) `bucket`

Nimmt eine Liste von **Instants** (Epoch oder RFC3339 mit Offset/Z) und gibt pro Input den Bucket aus.

Beispiele:

* `tzbucket bucket --tz Europe/Berlin --interval day --in fixtures/timestamps.txt --format json`
* `tzbucket bucket --tz Europe/Berlin --interval week --week-start monday --stdin --format json`

**Input-Formate (v0.1, bewusst simpel):**

* `epoch_ms` (Default) oder `epoch_s`
* `rfc3339` (z.B. `2026-03-29T00:15:00Z`)

> Hinweis: v0.1 akzeptiert **nur Instants** (mit Offset/UTC). Damit umgehen wir „ambiguous/nonexistent local time“ beim Parsen. Policies dazu kommen über `explain` (siehe unten).

#### B) `range`

Gibt eine **Bucket-Liste** für einen Zeitraum aus (hilfreich für Reports/ETL).

Beispiele:

* `tzbucket range --tz Europe/Berlin --interval day --start "2026-03-27T00:00:00Z" --end "2026-03-31T00:00:00Z" --format json`

Output: Buckets (start/end) in local + UTC + key.

#### C) `explain`

Erklärt DST-Edgecases für **lokale Zeitstrings ohne Offset** in einer TZ — inkl. Policy-Verhalten.

Beispiele:

* `tzbucket explain --tz Europe/Berlin --local "2026-03-29T02:30:00" --policy nonexistent=shift_forward`
* `tzbucket explain --tz Europe/Berlin --local "2026-10-25T02:30:00" --policy ambiguous=first`

**Policies (v0.1):**

* `nonexistent = error | shift_forward`
* `ambiguous = error | first | second`

Das ist wichtig, weil in Berlin am **29. März 2026 um 02:00** die Uhr auf **03:00** springt (02:xx existiert nicht). ([Time and Date][1])
Und am **25. Oktober 2026** wird die Stunde **02:00–02:59** doppelt durchlaufen. ([Time and Date][1])

---

## Definition: “kalenderbasierte Buckets”

Buckets sind **an lokalen Kalendergrenzen ausgerichtet**, nicht „fixed duration in seconds“.

* `day`: local `00:00:00` → nächster local `00:00:00`

  * kann **23h** (DST start) oder **25h** (DST end) in UTC-Dauer sein
* `week`: Start abhängig von `--week-start monday|sunday`, jeweils local `00:00`
* `month`: local erster Tag des Monats `00:00` → nächster Monat erster Tag `00:00`

---

## Output Contract (JSON, deterministisch)

Für `bucket` (pro Input-Zeitstempel):

```json
{
  "input": { "ts": "2026-03-29T00:15:00Z", "epoch_ms": 0 },
  "tz": "Europe/Berlin",
  "interval": "day",
  "bucket": {
    "key": "2026-03-29",
    "start_local": "2026-03-29T00:00:00+01:00",
    "end_local": "2026-03-30T00:00:00+02:00",
    "start_utc": "2026-03-28T23:00:00Z",
    "end_utc": "2026-03-29T22:00:00Z"
  }
}
```

**Stabilität:**

* Feldreihenfolge über Structs/serde (nicht HashMaps)
* Arrays **sortiert** (z.B. in `range` nach `start_utc`)

Für `range`: Array von `bucket` Objekten.

Für `lint` haben wir im MVP keinen eigenen Command — stattdessen sollen `explain` und klare Fehlercodes reichen.

---

## Tech-Vorgaben

### Sprache / Crates

* Rust workspace (wie Template):

  * `crates/tzbucket-core`
  * `crates/tzbucket-cli`
* CLI: `clap` v4.x ([Docs.rs][2])
* TZ-Handling: bevorzugt `chrono` + `chrono-tz` (IANA TZ Daten compile-time, unabhängig vom OS-Zoneinfo) ([Docs.rs][3])
  (Wichtig, damit es auch in minimalen Docker Images stabil läuft; `chrono::Local` hängt sonst am System/`/etc/localtime`.) ([The Rust Programming Language Forum][4])

### Fehler-/Exitcodes

* `0` OK
* `2` invalid input / parse error / policy error
* `3` runtime (I/O, unexpected)

### Performance (MVP)

* `bucket` soll streaming-fähig sein (stdin → stdout) ohne alles im Speicher zu halten.

---

## Testplan (Fixtures + Golden)

Lege `fixtures/` und `golden/` an (Template-Pattern).

### Pflicht-Fixtures

1. **Europe/Berlin DST Start 2026**

* Berlin DST Start: **29. März 2026**, Sprung **02:00 → 03:00**. ([Time and Date][1])
  Fixtures:
* Input timestamps (UTC) rund um den Wechsel, z.B.:

  * `2026-03-28T22:30:00Z`
  * `2026-03-28T23:30:00Z`
  * `2026-03-29T00:30:00Z`
  * `2026-03-29T21:30:00Z`
    Golden: erwartete day buckets (mit 23h-Dauer an diesem Tag).

2. **Europe/Berlin DST End 2026**

* Berlin DST End: **25. Oktober 2026**, „Fall back“. ([Time and Date][1])
  Inputs rund um den Tag, Golden: day bucket mit 25h-Dauer.

3. **America/New_York DST Start/End 2026 (smoke)**

* Start **8. März 2026 02:00 → 03:00**, Ende **1. Nov 2026**. ([Time and Date][5])
  Nur 2–3 Inputs, um zu zeigen: multi-tz funktioniert.

4. `explain` Tests

* Berlin `2026-03-29T02:30:00` → nonexistent
* Berlin `2026-10-25T02:30:00` → ambiguous
  Golden: JSON Ergebnis abhängig von policy.

---

## Implementationsschritte

1. **Core Datenmodelle**

* `Interval` enum: `Day | Week | Month`
* `WeekStart` enum: `Monday | Sunday`
* `Policy` struct: `nonexistent`, `ambiguous`

2. **Core Algorithmus**

* Für gegebenen `Instant (UTC)`:

  * convert to `local` in TZ
  * derive bucket start in local (00:00 / week boundary / month boundary)
  * compute bucket end in local (next day/week/month boundary)
  * convert start/end back to UTC
* Achtung: start/end local conversion kann DST-Sprünge enthalten → muss über TZ-aware conversion laufen.

3. **CLI**

* clap commands: `bucket`, `range`, `explain`
* Input parsing: epoch_ms/epoch_s/rfc3339
* Output: `--format json|text`, default `text` (kurz/lesbar), JSON strikt nach Contract

4. **Golden Harness**

* tests starten CLI gegen fixtures und vergleichen JSON mit golden files.

---

## Deliverables

* `tzbucket` Binary (läuft auf Win/macOS/Linux)
* README:

  * What/Why
  * Quickstart + Beispiele
  * Output Contract
  * DST-Notes (Berlin/NY als Beispiele, mit konkreten Daten)
* CI: fmt, clippy, test, build matrix
* 1st Release Tag `v0.1.0` erzeugt Binaries

---

## Akzeptanzkriterien

* `tzbucket bucket --tz Europe/Berlin --interval day --in fixtures/berlin_dst_start_2026.txt --format json`

  * liefert JSON
  * golden test grün
  * zeigt, dass der **Bucket am 29.03.2026** in UTC **23h** lang ist (start/end_utc differieren um 23h)
* `tzbucket explain --tz Europe/Berlin --local "2026-03-29T02:30:00" --policy nonexistent=error` → Exitcode 2 + Fehlermeldung
* `range` produziert korrekt sortierte Bucket-Liste über DST-Grenzen

---

## Empfehlung: womit anfangen (konkret)

Für eine schnelle Implementierung: zuerst `bucket` (Instant → bucket), dann Berlin DST Tests, dann `range`, dann `explain`.
