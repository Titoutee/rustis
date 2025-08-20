# Rustis
Rustis is a compact, thread-safe, babytoy Redis server implementing  part of *standard RESP*, written in pure Rust.

## Support
Rustis provides a (yet simplified but) *robust and quite exhaustive interface to the Redis protocol*.
As part of implemented commands, which is what is expected at the end of the day by someone asking the question _"what does this server expose?"_, the supported commands are listed below:

Rustis server is limited to ***100*** concurrent clients for now, thus is prone to `panic`-ing *in case >100 clients ask for connection.*

- ***ECHO***
- ***SET/GET*** : SETs & GETs to & from the server's internal db
- ***INCR*** : increments only `RedisInt` (aka proprietary integers ) values and instantiates+increments when object not present in db
- ***MULTI*** : queue in several commands, to be consumed later on
- ***EXEC*** : consumes the queued commands

[For documentation on RESP](https://redis.io/docs/latest/develop/reference/protocol-spec/)

[For information on Redis](https://en.wikipedia.org/wiki/Redis)

## Concurrent database access
The shared database is accessed by clients using their _**individual client id**_.

_**For now**_, we mean that each db's entry is signed with the owner's (client) id separately from the corresponding key, meaning that the client id of the 
_inserter_ get embedded _within_ the key as a suffix, all flushed as a string ahead of time. Alternatively, on could bring the client id separately from the actual key (*still as a `RedisValue`*) in a suited struct.

In the following, each database _entry_ will be mentioned as an _**object**_.

## Appendix
### Commands usage
These commands are **not** formatted as **RESP** enforces, but rather as some sort of input a client may get them from the user before turning them into so

- ***ECHO*** : **ECHO lol** _(with* `lol` *being initially inserted)_
- ***SET*** : **SET lol val**, _sets `lol` to `val`, or inserts `lol` with value `val` if not in already_
- _**SET** (with expiry)_ : **SET lol val x**, _with an expiry time_
- ***GET*** : **GET lol**, _gets the value associated with `lol` if exists_
- ***MULTI*** : **MULTI**, _prepares the queue for upcoming commands_
- ***EXEC*** : **EXEC**, _executes all the commands added to the only queue by preceding calls to MULTI_

### Client cleanup
_When a client disconnects_, inherently to the **database centralization** model, one must perform some **object cleanup** procedure, which simply goes through the object references updated at each _`insert`_ or _`remove`_ operation from the aforementioned client.

### How do I test it out?

Basically, take or craft any python client or *telnet* script to communicate with the given Redis server. As an example, `client.py` performs some payload tests to check for server responses. 

The default used addr/port config is **`localhost:6378`**.
A proper, clean client will be provided in the soon future, someone feel free to pull request if have one at reach, I don't write idiomatic python on my part... /xp/
