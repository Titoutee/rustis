# Rustis
Rustis is a compact, thread-safe, babytoy Redis server implementing  part of *standard RESP*, written in pure Rust.

## Support
Rustis provides a (yet simplified but) robust and quite exhaustive interface to the Redis protocol.
As part of implemented commands, which is what is esepcted for someone asking the question "what does this server expose?", the supported commands are listed below:

Rustis is limited to ***100*** concurrent clients for now.

- ***ECHO***
- ***SET/GET*** : SETs & GETs to & from the server's internal db
- ***INCR*** : increments only `RedisInt` (aka proprietary integers ) values and instantiates+increments when object not present in db
- ***MULTI*** : queue in several commands, to be consumed later on
- ***EXEC*** : consumes the queued commands
- ******


Rustis adds additional support for entry expiry, ...

[For documentation on RESP](https://redis.io/docs/latest/develop/reference/protocol-spec/)

[For information on Redis](https://en.wikipedia.org/wiki/Redis)


## Appendix

### How do I test it out?

Basically, take or craft any python client or *telnet* script to communicate with the given Redis server. As an example, `client.py` performs some payload tests to check for server responses. 

The default used addr/port config is **`localhost:6378`**.
A proper, clean client will be provided in the soon future, someone feel free to pull request if have one at reach.
