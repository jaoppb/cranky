# Contributing to Cranky

This is the procedure: how to set up, file an issue, open a pull request and cut a release. The rules it relies on (conventions, hard constraints, commands, labels, branch and commit naming, the gate) are in [`AGENTS.md`](AGENTS.md), and apply to humans and agents alike. Read it before your first change.

## Setup

1. Install [Nix](https://nixos.org/download/) with flakes enabled, and clone the repository.
2. Enter the devShell, either by hand or through [direnv](https://direnv.net/):

   ```bash
   nix develop    # or, once: direnv allow
   ```

   The devShell provides everything the gate needs, at the versions `flake.lock` pins: the Rust toolchain from `rust-toolchain.toml` (1.100.0), `just`, `lefthook`, `cargo-deny`, `cargo-llvm-cov`, `actionlint`, `shellcheck`, `tools` and `gh`, and `dbus-daemon`, `sway` and `grim` for the integration tests. Nothing else needs installing.

   Every `just` recipe refuses to run outside the devShell (see "Commands" in `AGENTS.md`). From outside, run `nix develop -c just <recipe>`.
3. Install the git hooks:

   ```bash
   just setup
   ```

4. Check that everything works with `just check`.

The tool pins move only through `just flake-update`, in a pull request of its own.

### GitHub access for the audit recipes

`just issues-check`, `just repo-check` and `just project-check` read GitHub through `gh`, so run `gh auth login` first. `just repo-check` also audits the merge settings, which only an account with push access can see. `just project-sync` writes to the Project board and needs the `project` scope: `gh auth refresh -s project`. A contributor without that scope can skip these recipes; the maintainer runs them.

## Before you push

Run `just check`. Its `deps-check` step needs the network. When your change can affect coverage or the benchmarks, also run `just coverage-check` or `just bench-check`: CI runs both. When it touches `tools.toml`, the labels blocks of `AGENTS.md` and this file, or the issue forms, run `just repo-check` too.

## Filing an issue

Open a new issue and pick the form for its type: Feature, Fix, Chore, Docs, Refactor or Spike. Each form applies its `type:*` label and title prefix. Blank issues are disabled. Fill in:

- **Description:** what and why.
- **Dependencies:** the issues that must close first, or "None.".
- **Definition of Done:** checkable outcomes.

A form cannot apply an `area:*` label or set a milestone, so the issue still needs triage after you file it. The maintainer adds one `area:*` label, the release milestone and the parent epic (see "Issues, branches and commits" in `AGENTS.md`). Until then the **Tools** workflow fails its `issues-check` run, and GitHub may email you that the run failed. That is expected, and there is nothing for you to do.

## Issue lifecycle

Every issue is a card of the public Project board [Cranky](https://github.com/users/jaoppb/projects/7). New issues are added to it by the board's built-in auto-add workflow. The board must stay **public**: CI's `issues-check` reads it with the workflow's default token, and that job fails if the board is private.

The card's Status follows the `builtin:cosmo` preset that `tools.toml` names:

| Status | When |
| --- | --- |
| Backlog | The default, and where an issue stays until it is triaged and unblocked |
| Ready | One `type:*` label, an `area:*` label, a milestone, the board fields Priority (`P0`–`P3`) and Effort (at most 8), no open blocker, and no unstarted parent that is not an epic |
| In progress | Moved here by hand when work starts |
| In review | A started issue with a pull request linked to it |
| Done | The issue is closed |

Epic cards are never moved by the sync; they are moved by hand.

Nothing in CI moves the cards. The maintainer runs, locally:

- `just project-check`, which prints the moves the cards need and the frozen cards, and writes nothing. `just project-check --explain <issue>` shows why a card stays where it is.
- `just project-sync`, which writes the moves.

A local `just issues-check` also reports an issue missing from the board. CI's run skips that rule, because nothing in CI adds cards.

## Pull requests

1. Branch from `main` with the name `AGENTS.md` gives (`<type>/<issue>`).
2. Commit following `AGENTS.md`, and make sure `just check` passes.
3. Open the pull request against `main`. The template asks for `Closes #<n>`, what changed, notes for the reviewer, and the test plan. Dependent changes can go out as a stack of pull requests, each based on the one below it.
4. The 11 required checks must be green. The `test` job can fail once on the runner without a code cause; re-run the failed job once before investigating.
5. The maintainer merges, with a merge commit.

## Releasing

Maintainers only. A release is a version bump merged like any other change, then a tag:

1. `just bump-version X.Y.Z` writes the version into `Cargo.toml`, `Cargo.lock` and `contrib/arch/PKGBUILD*`. It touches neither git nor GitHub, and refuses a version that is not greater than the current one.
2. Commit that on a branch, open a pull request, and merge it once the checks pass.
3. Push the tag `vX.Y.Z` on the merged commit of `main`.

`release.yml` then checks that the tag is a version, is on `main` and equals the Cargo version. It runs the full CI gate, publishes the GitHub release with generated notes, and publishes the `cranky` and `cranky-bin` AUR packages.

If something goes wrong:
- A rejected tag publishes nothing: delete it and push it again on the right commit.
- A later failure is fixed by re-running the failed jobs on the same tag.
- Once the GitHub release exists, a fault that needs a code change gets a new patch version, never a moved tag.

## Labels

The labels `tools.toml` declares. `just repo-check` fails when this list, the one in `AGENTS.md` and `tools.toml` disagree.

<!-- labels:start -->
- `type:feat`
- `type:fix`
- `type:chore`
- `type:docs`
- `type:refactor`
- `type:spike`
- `area:wayland`
- `area:rendering`
- `area:scripting`
- `area:modules`
- `area:tooling`
- `area:docs`
- `epic`
<!-- labels:end -->
