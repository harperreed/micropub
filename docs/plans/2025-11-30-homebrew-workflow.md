# Homebrew Release Workflow Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Automate Homebrew tap updates with pre-compiled bottles when releases are created

**Architecture:** Two-workflow approach - modified release workflow builds cross-platform binaries, new homebrew workflow builds bottles and updates formula on release events

**Tech Stack:** GitHub Actions, Rust cross-compilation, Homebrew formula generation, git automation

---

## Prerequisites

Before starting implementation, set up the required secret:

1. Create Personal Access Token with `repo` scope for `harperreed/homebrew-tap`
2. Add as `HOMEBREW_TAP_TOKEN` secret in micropub repository settings
3. Verify secret exists: Settings → Secrets → Actions → HOMEBREW_TAP_TOKEN

---

## Task 1: Modify Release Workflow for Cross-Compilation

**Files:**
- Modify: `.github/workflows/release.yml:36-76`

**Goal:** Update release workflow to build binaries for multiple platforms and attach to GitHub Release

### Step 1: Add build matrix job before release job

**Action:** Insert new `build-binaries` job after `check-version` job

**Code to add after line 32:**

```yaml
  build-binaries:
    needs: check-version
    if: needs.check-version.outputs.version_changed == 'true'
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: x86_64-apple-darwin
            name: micropub-x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
            name: micropub-aarch64-apple-darwin
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            name: micropub-x86_64-unknown-linux-gnu
    runs-on: ${{ matrix.os }}
    steps:
      - name: Checkout code
        uses: actions/checkout@v3

      - name: Set up Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}
          override: true

      - name: Build release binary
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package binary
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../${{ matrix.name }}.tar.gz micropub
          cd ../../..

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: ${{ matrix.name }}
          path: ${{ matrix.name }}.tar.gz
```

### Step 2: Update release job to depend on build-binaries

**Action:** Modify `release` job to download and attach binaries

**Replace the entire `release` job (lines 33-76) with:**

```yaml
  release:
    needs: [check-version, build-binaries]
    if: needs.check-version.outputs.version_changed == 'true'
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v3

      - name: Set up Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Run tests
        run: cargo test

      - name: Download all artifacts
        uses: actions/download-artifact@v3
        with:
          path: artifacts

      - name: Display artifact structure
        run: ls -R artifacts/

      - name: Create tag
        run: |
          VERSION=${{ needs.check-version.outputs.version }}
          git config --local user.email "github-actions[bot]@users.noreply.github.com"
          git config --local user.name "github-actions[bot]"
          if git rev-parse "v$VERSION" >/dev/null 2>&1; then
            echo "Tag v$VERSION already exists, skipping tag creation"
          else
            git tag -a "v$VERSION" -m "Release v$VERSION"
            git push origin "v$VERSION"
          fi

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          tag_name: v${{ needs.check-version.outputs.version }}
          name: Release v${{ needs.check-version.outputs.version }}
          draft: false
          prerelease: false
          generate_release_notes: true
          files: |
            artifacts/micropub-x86_64-apple-darwin/micropub-x86_64-apple-darwin.tar.gz
            artifacts/micropub-aarch64-apple-darwin/micropub-aarch64-apple-darwin.tar.gz
            artifacts/micropub-x86_64-unknown-linux-gnu/micropub-x86_64-unknown-linux-gnu.tar.gz
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Publish to crates.io
        uses: katyo/publish-crates@v2
        with:
          registry-token: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

### Step 3: Test the modified workflow locally

**Action:** Validate YAML syntax

Run: `yamllint .github/workflows/release.yml` (or use online YAML validator)
Expected: No syntax errors

### Step 4: Commit the changes

```bash
git add .github/workflows/release.yml
git commit -m "feat: add cross-platform binary builds to release workflow

- Build binaries for macOS (Intel + ARM) and Linux
- Package as tar.gz artifacts
- Attach to GitHub Release
- Make tag creation idempotent (skip if exists)"
```

---

## Task 2: Create Homebrew Update Workflow

**Files:**
- Create: `.github/workflows/homebrew.yml`

**Goal:** Create workflow that builds bottles and updates Homebrew formula on release

### Step 1: Create the workflow file

**Action:** Create new file `.github/workflows/homebrew.yml`

**Complete file contents:**

```yaml
name: Update Homebrew Tap

on:
  release:
    types: [published]

jobs:
  build-bottles:
    strategy:
      matrix:
        include:
          - os: macos-13  # Intel Mac
            arch: x86_64
            bottle_tag: ventura
          - os: macos-14  # Apple Silicon
            arch: arm64
            bottle_tag: arm64_sonoma
    runs-on: ${{ matrix.os }}
    steps:
      - name: Checkout code
        uses: actions/checkout@v3
        with:
          ref: ${{ github.event.release.tag_name }}

      - name: Set up Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Build release
        run: cargo build --release

      - name: Create bottle
        run: |
          VERSION="${{ github.event.release.tag_name }}"
          VERSION="${VERSION#v}"  # Remove 'v' prefix
          BOTTLE_NAME="micropub-${VERSION}.${{ matrix.bottle_tag }}.bottle.tar.gz"

          cd target/release
          tar czf "../../${BOTTLE_NAME}" micropub
          cd ../..

          # Calculate SHA256
          SHA256=$(shasum -a 256 "${BOTTLE_NAME}" | awk '{print $1}')
          echo "sha256=${SHA256}" >> $GITHUB_OUTPUT
          echo "bottle_name=${BOTTLE_NAME}" >> $GITHUB_OUTPUT
        id: bottle

      - name: Upload bottle
        uses: actions/upload-artifact@v3
        with:
          name: bottle-${{ matrix.arch }}
          path: ${{ steps.bottle.outputs.bottle_name }}

      - name: Save bottle info
        run: |
          echo "${{ matrix.bottle_tag }}:${{ steps.bottle.outputs.sha256 }}" > bottle-${{ matrix.arch }}.txt

      - name: Upload bottle info
        uses: actions/upload-artifact@v3
        with:
          name: bottle-info-${{ matrix.arch }}
          path: bottle-${{ matrix.arch }}.txt

  update-formula:
    needs: build-bottles
    runs-on: ubuntu-latest
    steps:
      - name: Get release version
        id: version
        run: |
          VERSION="${{ github.event.release.tag_name }}"
          VERSION="${VERSION#v}"
          echo "version=${VERSION}" >> $GITHUB_OUTPUT

      - name: Download source tarball and calculate SHA256
        id: source
        run: |
          VERSION="${{ steps.version.outputs.version }}"
          URL="https://github.com/harperreed/micropub/archive/refs/tags/v${VERSION}.tar.gz"
          curl -L -o source.tar.gz "${URL}"
          SHA256=$(shasum -a 256 source.tar.gz | awk '{print $1}')
          echo "sha256=${SHA256}" >> $GITHUB_OUTPUT

      - name: Download bottle info
        uses: actions/download-artifact@v3
        with:
          path: bottle-info

      - name: Read bottle SHA256s
        id: bottles
        run: |
          X86_64_INFO=$(cat bottle-info/bottle-info-x86_64/bottle-x86_64.txt)
          ARM64_INFO=$(cat bottle-info/bottle-info-arm64/bottle-arm64.txt)

          X86_64_TAG=$(echo "$X86_64_INFO" | cut -d':' -f1)
          X86_64_SHA=$(echo "$X86_64_INFO" | cut -d':' -f2)

          ARM64_TAG=$(echo "$ARM64_INFO" | cut -d':' -f1)
          ARM64_SHA=$(echo "$ARM64_INFO" | cut -d':' -f2)

          echo "x86_64_tag=${X86_64_TAG}" >> $GITHUB_OUTPUT
          echo "x86_64_sha=${X86_64_SHA}" >> $GITHUB_OUTPUT
          echo "arm64_tag=${ARM64_TAG}" >> $GITHUB_OUTPUT
          echo "arm64_sha=${ARM64_SHA}" >> $GITHUB_OUTPUT

      - name: Generate formula
        run: |
          VERSION="${{ steps.version.outputs.version }}"
          SOURCE_SHA="${{ steps.source.outputs.sha256 }}"
          X86_64_TAG="${{ steps.bottles.outputs.x86_64_tag }}"
          X86_64_SHA="${{ steps.bottles.outputs.x86_64_sha }}"
          ARM64_TAG="${{ steps.bottles.outputs.arm64_tag }}"
          ARM64_SHA="${{ steps.bottles.outputs.arm64_sha }}"

          cat > micropub.rb << EOF
          class Micropub < Formula
            desc "Ultra-compliant Micropub CLI with MCP server support"
            homepage "https://github.com/harperreed/micropub"
            url "https://github.com/harperreed/micropub/archive/refs/tags/v${VERSION}.tar.gz"
            sha256 "${SOURCE_SHA}"
            license "MIT"

            depends_on "rust" => :build

            bottle do
              root_url "https://github.com/harperreed/micropub/releases/download/v${VERSION}"
              sha256 cellar: :any_skip_relocation, ${ARM64_TAG}: "${ARM64_SHA}"
              sha256 cellar: :any_skip_relocation, ${X86_64_TAG}: "${X86_64_SHA}"
            end

            def install
              system "cargo", "install", *std_cargo_args
            end

            test do
              assert_match version.to_s, shell_output("#{bin}/micropub --version")
            end
          end
          EOF

      - name: Checkout homebrew-tap
        uses: actions/checkout@v3
        with:
          repository: harperreed/homebrew-tap
          token: ${{ secrets.HOMEBREW_TAP_TOKEN }}
          path: homebrew-tap

      - name: Update formula in tap
        run: |
          mkdir -p homebrew-tap/Formula
          cp micropub.rb homebrew-tap/Formula/micropub.rb

      - name: Commit and push formula
        run: |
          cd homebrew-tap
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Formula/micropub.rb
          git commit -m "Update micropub to ${{ steps.version.outputs.version }}"
          git push

      - name: Upload bottles to release
        uses: softprops/action-gh-release@v1
        with:
          tag_name: v${{ steps.version.outputs.version }}
          files: |
            bottle-info/bottle-x86_64/*
            bottle-info/bottle-arm64/*
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Step 2: Validate workflow syntax

Run: Use GitHub's workflow validator or `yamllint .github/workflows/homebrew.yml`
Expected: No syntax errors

### Step 3: Commit the workflow

```bash
git add .github/workflows/homebrew.yml
git commit -m "feat: add Homebrew tap update workflow

- Triggers on release publish events
- Builds bottles for macOS (Intel + ARM)
- Calculates SHA256 for source and bottles
- Generates and updates Formula/micropub.rb
- Auto-commits to harperreed/homebrew-tap"
```

---

## Task 3: Test the Complete Workflow

**Goal:** Verify the workflows work together correctly

### Step 1: Push changes to trigger test

```bash
git push origin feature/homebrew-workflow
```

### Step 2: Create PR and test with version bump

**Action:** Create a test by bumping a patch version

1. Create PR from `feature/homebrew-workflow`
2. In PR, modify `Cargo.toml` version (e.g., `0.2.0` → `0.2.1-test`)
3. Merge PR to main
4. Watch workflows in Actions tab

Expected sequence:
1. Release workflow triggers on Cargo.toml change
2. Builds binaries for 3 platforms
3. Creates tag v0.2.1-test
4. Creates GitHub Release with binaries
5. Homebrew workflow triggers on release
6. Builds bottles for macOS
7. Updates formula in homebrew-tap

### Step 3: Verify formula was updated

```bash
# Check homebrew-tap repository
curl https://raw.githubusercontent.com/harperreed/homebrew-tap/main/Formula/micropub.rb
```

Expected: Formula exists with correct version and SHA256s

### Step 4: Test installation locally

```bash
# Add tap if not already added
brew tap harperreed/tap

# Install (or upgrade if already installed)
brew install micropub
# OR
brew upgrade micropub

# Verify version
micropub --version
```

Expected: Shows new version (0.2.1-test)

### Step 5: Clean up test release (if desired)

```bash
# Delete test release via GitHub UI
# Delete test tag
git push origin :refs/tags/v0.2.1-test

# Revert version in Cargo.toml back to 0.2.0
```

---

## Task 4: Update Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/plans/2025-11-30-homebrew-release-workflow-design.md`

### Step 1: Add Homebrew installation to README

**Action:** Find the installation section in README.md and add Homebrew instructions

**Code to add:**

```markdown
### Homebrew (macOS)

```bash
brew tap harperreed/tap
brew install micropub
```

### From Source

```bash
cargo install micropub
```
```

### Step 2: Update design doc with implementation status

**Action:** Update the success criteria in the design doc

Find the "Success Criteria" section and update:

```markdown
## Success Criteria

- [x] Design validated with stakeholder
- [x] Workflows implemented and tested
- [x] Formula successfully updates on release
- [x] Users can install via `brew install harperreed/tap/micropub`
- [x] Bottles work on both Intel and ARM Macs
- [x] Documentation updated with installation instructions
```

### Step 3: Commit documentation updates

```bash
git add README.md docs/plans/2025-11-30-homebrew-release-workflow-design.md
git commit -m "docs: add Homebrew installation instructions

- Add brew install command to README
- Mark design success criteria as complete"
```

---

## Task 5: Final Cleanup and Merge

**Goal:** Clean up and merge the feature branch

### Step 1: Push all commits

```bash
git push origin feature/homebrew-workflow
```

### Step 2: Create pull request

Create PR with description:
```
## Homebrew Release Automation

Implements automated Homebrew tap updates with bottle builds.

### Changes
- Modified release workflow for cross-platform binary builds
- Added Homebrew workflow for bottle creation and formula updates
- Updated documentation with Homebrew installation instructions

### Testing
- [x] Workflows validated with test release
- [x] Formula successfully updated in homebrew-tap
- [x] Installation tested on macOS (Intel and ARM)

### How it works
1. Push tag → Release workflow builds binaries
2. Release created → Homebrew workflow builds bottles
3. Formula auto-updated → Users can `brew upgrade micropub`

Closes #XX (if there's an issue)
```

### Step 3: Merge and verify

After PR approval:
1. Merge PR
2. Workflow will NOT trigger (already on v0.2.0)
3. Next version bump will trigger full flow

### Step 4: Clean up worktree

**REQUIRED:** Use @superpowers:finishing-a-development-branch skill

---

## Verification Checklist

After implementation, verify:

- [ ] Modified release workflow builds 3 platform binaries
- [ ] GitHub releases include binary attachments
- [ ] Homebrew workflow triggers on release events
- [ ] Bottles are built for macOS Intel and ARM
- [ ] Formula is auto-updated in homebrew-tap with correct SHA256s
- [ ] `brew install harperreed/tap/micropub` works
- [ ] Installation uses bottles (fast, no compilation)
- [ ] Documentation includes Homebrew installation instructions

---

## Troubleshooting

### Workflow fails with "tag already exists"
**Solution:** Tag creation is now idempotent - re-run workflow

### Homebrew workflow doesn't trigger
**Check:**
- Release must be "published" (not draft)
- Workflow file must be on main branch before release

### Formula has wrong SHA256
**Cause:** Source tarball SHA calculated before upload completes
**Solution:** GitHub auto-generates tarballs, wait 30s and re-run

### Bottle installation fails
**Check:**
- Bottle names match formula bottle block
- Bottles uploaded to correct release
- SHA256 matches actual bottle file

### HOMEBREW_TAP_TOKEN errors
**Check:**
- Secret exists in repository settings
- Token has `repo` scope
- Token hasn't expired

---

## Notes for Implementer

1. **Don't skip steps** - Each step builds on previous
2. **Test incrementally** - Commit after each task
3. **Use test versions** - Don't pollute production with failed attempts
4. **Check Actions tab** - Workflows show detailed logs
5. **Bottles are cached** - Homebrew caches bottles, may need `brew update --force`

**Time estimate:** 60-90 minutes total (excluding test release waiting time)
