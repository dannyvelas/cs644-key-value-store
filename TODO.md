## behavior
- [x] add compact function
- [ ] make compact function get called by signal
- [ ] print process id at start of session for user

## correctness
- [x] make server handle Ctrl+C gracefully
- [x] allow use to exit session by Ctrl+d or "exit" command
- [ ] remove all the print statements
- [ ] close epoll?
- [ ] make sure error handling like no key found is graceful

## cosmetic
- [x] probably encapsulate file reading in its own class
- [ ] make signal logic cleaner in `main.rs`
- [ ] use macro for error handling
- [ ] remove duplication of logic in disk/map.rs of pattern of open+lock+someop+unlock
- [ ] remove duplication of logic in disk/reader.rs when reading the key-value sizes
- [ ] remove duplication of logic in `append_key` in disk/map.rs when creating key-value pairs

## subjective
- [ ] make key-value slots 2 bytes instead of 4
