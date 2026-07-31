# GOTCHAS

Non-obvious discoveries for this repo — things that cost real debugging time and
are not captured in the code, tests, or commit messages. Add an entry the moment
you hit one; this file is the repo's shared debugging memory.

Format per entry:

## <symptom in a few words>

- **Symptom:** what you saw
- **Cause:** the real reason
- **Fix / avoid:** what to do

---

## The TUI 404s on every route against a server that is clearly running

- **Symptom:** `just server` reports "Server starting successfully", `curl 127.0.0.1:<port>/health` returns 200, but the TUI gets 404 on every request. No error anywhere says the two are talking to different processes.
- **Cause:** a process bound to IPv6 `*:PORT` and fido-server bound to IPv4 `127.0.0.1:PORT` coexist with **no** "address already in use" error, because they are different address families. `localhost` resolves to `::1` first on macOS, so any client using `localhost` reaches the *other* process. Hit in practice with narrator-ai on port 3000.
- **Fix / avoid:** always use `127.0.0.1` in client URLs and config, never `localhost`. Check with `lsof -nP -iTCP:<port> -sTCP:LISTEN`, which lists **both** listeners; a bare `curl localhost` will not reveal the conflict. The dev default moved to 4747 (stint 0031) to stay clear of the crowded 3000 range, and `just server` now preflights the port and names the process holding it.

## `just server` dies on a required environment variable that no example file mentions

- **Symptom:** `FATAL: Missing required configuration: ENVIRONMENT (or RUST_ENV) must be set to 'production' or 'development'`, while `just e2e-tui` passes happily.
- **Cause:** `ENVIRONMENT` is required at startup and was documented in neither `.env` nor `.env.example`. `scripts/e2e_tui.sh` sets it internally, so the automated path never exercised the manual one. CI stayed green while the documented manual path was broken for everyone.
- **Fix / avoid:** it is in `.env.example` now, and `just server` sets it explicitly. More generally: when a harness sets env vars inline, the manual path is untested by definition — mirror anything required into `.env.example`.

## A file is committed despite being in .gitignore, and `git status` says nothing

- **Symptom:** `wtp remove` refuses with "contains modified or untracked files" over a path that `.gitignore` covers. `git status` shows a clean tree.
- **Cause:** `.gitignore` does not untrack files that were already committed before the rule existed. The file stays tracked forever and reads as clean, so nothing surfaces it. `.stint/tasks/0018` sat in every commit this way while the other 29 task files were correctly ignored.
- **Fix / avoid:** verify with `git ls-files <path>`, never `git status`. To fix, `git rm --cached <path>`. Note that pulling such a commit **deletes the file from other working trees**, including your own on the next pull, because git removes files a commit untracks even when they are newly ignored — back up local-only content first.

## An e2e assertion passes without testing anything

- **Symptom:** a `wait_for`/`grep` assertion on the TUI pane succeeds even when the thing under test never rendered.
- **Cause:** the string also appears in persistent chrome. Scenario 6 asserted the repo browser was open by grepping for "Browse repos", which the global footer renders on every frame as `b: Browse repos`, so the assertion was satisfied before the popup existed.
- **Fix / avoid:** assert on strings only the surface under test renders. For the repo browser that is the filter placeholder `type to filter`. When adding a pane assertion, check the footer and rail text first.

## e2e times out on a branch that changed only documentation

- **Symptom:** `just e2e-tui` fails at a different scenario on each run, always as a `wait_for` timeout, while server logs are clean and the database is seeded correctly.
- **Cause:** two different things that look identical, and the distinction matters:
  - a **race** — the wait keys off a signal that appears too early. Scenario 7 waited for "Fido", which is painted on the first frame *before* session restore (frame one renders before network startup work by design), so the next keypress arrived before the app was on the Main screen. Fails on an idle machine too.
  - **contention** — every `wait_for` has a fixed 15s budget (75 tries x 0.2s). Under heavy load (observed at load average 78+ from unrelated cargo builds in another repo on the same machine) a debug-build TUI cannot boot and fetch inside it.
- **Fix / avoid:** check `uptime` before debugging. If load is high, it is contention — do not touch the test. If the machine is idle, it is a race: move the wait to a post-restore marker such as the board title, and do **not** raise the retry count, which only delays discovering the same failure.
