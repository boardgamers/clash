# Status-phase battle restrictions — viewer 0.1.230 / engine 0.4.162

Defenders receive the battle movement restriction only during their own regular
turn, not during a status-phase incident. The enclosing status-phase event is
checked across the nested event stack: the retained current-player index alone
cannot identify a regular turn while combat resolves.

The rules distinguish the same-turn battle restriction (page 14) from the
status-phase free advance and its possible event (page 18). Regular-turn incident
defenders still remain restricted for the rest of that turn; unrelated groups
can move normally.

Two synthetic native regressions cover a Free Advance → Famine → barbarian battle
and the corresponding paid-advance incident. The former finishes the status
phase and moves the survivor next age; the latter checks that fighters remain
blocked across actions while another group moves freely. Two serialized WASM
regressions run in `web-client`'s `npm test` (and therefore Web viewer CI).
They can also run against an extracted release package:

```sh
CLASH_WASM=/absolute/path/package/server.js node --test scripts/status-phase-battle.test.cjs
```

No save migration, historical repair, turn-start cleanup, live-game writes or
achievement changes are included. Catalog revision 8 (46 definitions), artwork
revision 4 and all 60 named assets are preserved.
