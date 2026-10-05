# Player Manager

A [Calagopus Panel](https://calagopus.com) extension for managing the players of game servers. It is built
around a small per-game module system; the supported games today are **Minecraft: Java Edition**
(vanilla, Paper/Spigot, Fabric, Forge and other servers that keep the vanilla player files) and
**Minecraft: Bedrock Dedicated Server**. Open **Players** in the server sidebar to see who is online, kick
or ban them, and edit the whitelist, operators and bans without touching the console or the server files.

Package name: `dev.caloptreyx.playermanager` · Requires panel `>=1.2.3`

## Features

- **Online players**: the current player list with count and slots, read from the server's own `list`
  command. Java servers also return each player's UUID. Kick, ban, op/deop and whitelist players from
  their row.
- **Whitelist / allowlist**: add and remove players and turn the whitelist on or off.
- **Operators**: Java permission levels 1 to 4 and "bypass player limit"; Bedrock `operator`, `member`
  and `visitor` levels.
- **Bans and IP bans** (Java): ban with a reason, see who banned whom and when, pardon.
- **Name lookup**: players the server has seen before are suggested while typing. Java UUIDs come from
  `usercache.json`, the Mojang API, or the offline-mode UUID when `online-mode=false`. Bedrock XUIDs come
  from the allowlist, the console log (`Player connected`) or the GeyserMC API.
- **Game detection**: the extension recognises the game from the server's files (and its egg/image name),
  so the same page works for every supported game. Tabs, forms and actions follow what the detected game
  supports.

## How changes are applied

| Server | State | How |
|---|---|---|
| Java | running | Console commands (`whitelist add`, `op`, `ban`, `pardon`, `ban-ip`, `kick`...). The server updates its own files. |
| Java | offline | `whitelist.json`, `ops.json`, `banned-players.json`, `banned-ips.json` and `server.properties` are edited through Wings. |
| Bedrock | running | `allowlist.json` / `permissions.json` are edited, then `allowlist reload` / `permission reload` is sent. Turning the allowlist on or off edits `server.properties` and needs a restart. |
| Bedrock | offline | The files are edited. |

While the server is starting or stopping, changes are refused. Operator levels and "bypass player limit"
can only be chosen while a Java server is offline; the `op` command always uses the server's
`op-permission-level`.

Bedrock Dedicated Server has no ban list, so Bedrock servers get a tab explaining that instead of the ban
lists.

Refreshing the online list runs `list` in the console (`minecraft:list uuids` on Java), so it shows up in
the console output. The list is not polled in the background.

## Permissions

The extension uses the panel's own permissions; it adds none.

| Action | Permission |
|---|---|
| Open the page, see whitelist, operators and bans | `files.read-content` |
| See online players | `control.console` and `control.read-console` |
| Change anything while a Java server is running, kick | `control.console` |
| Change anything while the server is offline, any Bedrock change | `files.create` (the panel's file write permission) |
| Bedrock changes while running (the reload command) | additionally `control.console` |

Commands are logged as `server:console.command` and file edits as `server:file.write`, the same activity
events the panel's console and file editor create.

## Installation

Download `dev_caloptreyx_playermanager.c7s.zip` from the latest release and either upload it under
**Admin → Extensions** or put it in your heavy image's `build/extensions/` directory and run
`docker compose restart web`. Extensions need the `:heavy` panel image or a development environment.
The extension has no database tables.

To build the zip from a checkout, run `python3 scripts/package.py` (writes `dist/`).

## API

All routes live under `/api/client/servers/{server}/player-manager`:

| Route | Purpose |
|---|---|
| `GET /` | Detected game and what it supports, server state, settings, its player lists, known players |
| `GET /online` | Online players (Minecraft: runs `list`) |
| `POST`, `DELETE /lists/{kind}` | Add or remove an entry; `kind` is `whitelist`, `operators`, `bans` or `ip_bans` |
| `PUT /whitelist` | Turn the whitelist on or off |
| `POST /kick` | Kick an online player |

`GET /` returns a `game` descriptor: its id and family, the lists it has (with the fields each list
accepts in the current server state, such as operator levels or a ban reason), the name and id patterns
used for validation, and for every action the permissions it needs right now. The frontend renders from
that descriptor, so it needs no changes for a game whose wording fits the defaults.

## Adding a game

1. Backend: add a module under `src/games/` that implements the `Game` trait (`src/games/mod.rs`):
   a detection score from the server's root files and egg/image name, the descriptor (lists,
   patterns, capabilities), and the overview, online list and list/whitelist/kick actions it supports.
   Unsupported actions keep the trait's default implementations. Register it in `GAMES`.
2. Frontend (optional): add `frontend/src/games/<game>.ts` and register it in
   `frontend/src/games/index.ts` for a display name, avatars, wording overrides and notes. Without it the
   page uses generic wording and letter avatars.
3. If the game needs a list that is not one of the existing kinds, add the kind to `ListKind` on both
   sides.

## Development

The crate lives at the repository root, the frontend in `frontend/`. Every push runs the shared
extension check (`.github/workflows/check.yml`): typecheck, Biome, frontend build, the node tests in
`tests/` and `cargo test` against the newest panel release.

## License

MIT
