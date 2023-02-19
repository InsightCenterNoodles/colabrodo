# To Do

- [ ] Improve table handling
- [ ] Improve plot handling
- [ ] More testing (textures, etc)
- [ ] Server as client
- [ ] Better client interface
- [ ] HTTP server
- [ ] Clean up serde. At the moment we cannot use enums, and have to use optional structs. This breaks the spec if people misuse it, and bloats struct sizes.

Also would be good to clean up the bifurcation of message types. I'm doing it this way to allow smarter references, and the references mean different things for the clients and server, as well as the structure of updates, etc. But there has to be some way to merge them.