# HarborFeed

HarborFeed is a Rust library and small command-line utility for normalizing operational maritime feeds. It parses real-world text formats used around ports and vessel traffic systems:

- NMEA 0183 GPS sentences such as GGA, RMC, GLL, VTG, HDT, and ZDA
- AIS NMEA sentences such as AIVDM and AIVDO, including six-bit AIS payload decoding
- UN/EDIFACT interchanges used by port and cargo workflows, including COPRAR, COARRI, CUSCAR, BAPLIE, IFTMBC, and IFTMCS-style segment streams
- Tide/current station CSV exports with observed and predicted water levels

The library produces normalized events for vessel positions, vessel static data, container/cargo movements, port calls, and tide readings. It also runs consistency checks that are useful for ingest pipelines: bad NMEA checksums, impossible vessel coordinates, stale cargo references, draft-versus-port-depth mismatches, and contradictory EDIFACT segment sequences.

## Examples

```bash
cargo run -- samples/mixed_feed.txt
```

Library usage:

```rust
let feed = harborfeed::parse_feed_bytes(include_bytes!("samples/mixed_feed.txt"))?;
let findings = harborfeed::audit_feed(&feed);
```

## Fuzzing

The repository includes cargo-fuzz targets under `fuzz/` for the public parsers. They are part of the normal test strategy for malformed maritime feeds and do not participate in the production API.
