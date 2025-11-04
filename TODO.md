## behavior
- [x] add compact function
- [x] make compact function get called by signal
- [ ] print process id at start of session for user

## correctness
- [x] make server handle Ctrl+C gracefully
- [x] allow use to exit session by Ctrl+d or "exit" command
- [x] make sure signal handling is graceful
- [x] close epoll?
- [x] remove all the print statements
- [ ] make sure error handling like no key found is graceful

## cosmetic
- [x] probably encapsulate file reading in its own class
- [x] make sure that all cases of `libc::write` use the `safe_` equivalents in server.rs
- [x] make sure that all cases of `libc::read` use the `safe_` equivalents in server.rs
- [ ] make signal logic cleaner in `main.rs`
- [ ] use macro for error handling
- [ ] remove duplication of logic in disk/map.rs of pattern of open+lock+someop+unlock
- [ ] remove duplication of logic in disk/reader.rs when reading the key-value sizes
- [ ] remove duplication of logic in `append_key` in disk/map.rs when creating key-value pairs

## subjective
- [ ] make key-value slots 2 bytes instead of 4
