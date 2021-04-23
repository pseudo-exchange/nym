<div align="center">
  <h1>
    NYM Exchange
  </h1>
  <p>
  Auctions for NEAR Protocol Accounts
  </p>
</div>

## Building
Run:
```bash
./build.sh
```

## Testing
To test run:
```bash
cargo test --package auction_house -- --nocapture
```

## Changelog

### `0.2.0`

Upgrade near_sdk to 3.1.0, fix transfer contracts to utilize fully trustless transfer logic. extended workflow.

### `0.0.1`

Initial setup