# Mini introduction
### crates
- [client](../client): client executable, web client, client loop, graphics, etc
- [server](../server): server executable, web server, server game loop
- [mods/coremod](../mods/coremod): main (example) mod, compiles to `.dll` (windows) or `.so` (linux), has client/server stuff separated via features<br>
-> runtime folders in [client](../client/runtime)/[server](../server/runtime) hold all the extracted library files<br>
-> put mods in [server/mods](../server/mods) folder and include them in [mods.ron](../server/mods/profile.ron) 

### how to run
- compile the mods you changed using [build.bat](../mods/coremod/build.bat) or [build.sh](../mods/coremod/build.sh)
- start server with args specified in [main](../server/src/main.rs)
- start client with args specified in [main](../client/src/main.rs)

---
# Big Projects:
- ECS
- getting a screen & client side rendering
---
# TODO/Fixes:
- [x] clean up client mod loading
- [ ] cleanup of error handling
  - [x] client side
  - [ ] server side
- [x] utility functions that return the path to a lib (and have platform appropriate ending)
- [ ] checking whether all mods are valid after loading by checking for files and checking which platforms are supported
- [ ] better mod build script using probably python, make it platform  generic

---
# Some thoughts below:
# ecs networking system

### Entity
- ComponentA
- ComponentB

-> each component is unique by type<br>
-> generic `get_component<T>() -> Option<T>`

### Component has:
- ServerSide
- ClientSide

### Problem 1: client side movement of other players/enemies or general updating:
- A: interpolation between the last and current position data for moving objects
- B: extrapolation by accounting for velocity/acceleration from server
- C: actually running the server code, only needing to update on unpredictable inputs/randomness

### Problem 2: client side input
- A: send to server, server emits global event, do it client side<br>
  -> only input and rendering needed client side, but very laggy
- B: emulate action client side, "shadowing", waiting for server confirmation<br>
  -> less laggy, harder to sync

### Approach
- P1-A
- server runs at 20 tps
- sending interpolatable data every 5 frames or so
- only sending data of entities within range
- sending spontaneous events which are more important immediately
- considering grouping data to ease network load 
(use a "ModsData" packet that just groups a few until its full and then a 
second is used, ideally we only ever need one on schduled updates)