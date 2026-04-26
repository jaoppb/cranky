---
name: release-updater
description: Automates the release process by updating Cargo.toml version, tagging, pushing, waiting for GitHub release deployment, and updating arch PKGBUILD files.
---

# Release Updater Workflow

This skill automates the release process for the repository. When triggered, strictly follow these steps:

## Step 1: Ask for the New Version
1. Use the `ask_user` tool to ask the user: "What is the new version number? (e.g., 0.4.0)". Wait for their answer.

## Step 2: Update Cargo Version
1. Use `run_shell_command` to update the version in `Cargo.toml`:
   ```bash
   sed -i 's/^version = ".*"/version = "NEW_VERSION"/' Cargo.toml
   ```
   *(Replace NEW_VERSION with the user's input).*
2. Run `cargo check` to automatically update `Cargo.lock`.
3. Verify the changes using `git diff`.

## Step 3: Commit, Tag, and Push
1. Run `git add Cargo.toml Cargo.lock`.
2. Run `git commit -m "chore: release vNEW_VERSION"`.
3. Run `git tag vNEW_VERSION`.
4. Run `git push && git push --tags`.

## Step 4: Wait for Release Deployment
1. The pushed tag will trigger a GitHub Action (e.g., `release.yml`) to create a GitHub Release.
2. Use `run_shell_command` to poll until the release is fully deployed and artifacts are available. You can use a loop with `gh run watch` or `gh release view vNEW_VERSION` to ensure everything is ready.

## Step 5: Update PKGBUILD Files
1. Update `contrib/arch/PKGBUILD`, `contrib/arch/PKGBUILD-bin`, and `contrib/arch/PKGBUILD-git`:
   - Use `sed -i` to update `pkgver=NEW_VERSION` and `pkgrel=1`.
2. Update checksums for `PKGBUILD`:
   - Run `cd contrib/arch && makepkg -g -p PKGBUILD` to generate the new `sha256sums`.
   - Update the `sha256sums` array in `PKGBUILD` with the generated values using `sed` or `replace`.
3. Update checksums for `PKGBUILD-bin`:
   - Run `cd contrib/arch && makepkg -g -p PKGBUILD-bin` to generate the new `sha256sums`.
   - Update the `sha256sums` array in `PKGBUILD-bin`.
4. The `PKGBUILD-git` file uses `SKIP` for checksums, so it only requires the `pkgver` update.

## Step 6: Commit PKGBUILD Updates
1. Run `git add contrib/arch/PKGBUILD*`.
2. Run `git commit -m "chore: update PKGBUILDs for vNEW_VERSION"`.
3. Run `git push`.
