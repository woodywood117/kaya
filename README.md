# kaya
A message broker in Rust

## Usage
### Server
`kaya --addr <address> --port <port number>`

### Client
`kaya-cli --topic <topic> --addr <address> --port <port number>`

## Todo
- [x] Write initial server and client.
- [x] Clean up message protocol.
- [ ] Clean up topic handling logic.
- [ ] Make server distributed.
- [ ] Genericize Library for external client use.
- [ ] Store messages to disk for future reading.
- [ ] Allow for clients to start reading from specified point in time.
