# Darwinbots Modern

Darwinbots Modern is a cross-platform artificial-life simulator inspired by Darwinbots 2. Robots execute compatible plain-text DNA, sense and interact with a shared world, reproduce, mutate, and compete for energy while the simulation runs as quickly as the selected hardware allows.

The modern application combines a Rust simulation engine, portable `wgpu` compute acceleration, a permanent CPU fallback, a headless CLI, and a .NET 10/Avalonia desktop interface. Windows 10 is the primary usability target; CI also builds and tests Linux x64, Intel macOS, and Apple silicon macOS packages.

> **Completion-branch status:** All known required code gaps in the agreed functional offline scope were closed on October 21, 2026. Critical Windows workflows, original-file comparisons, physical GPU checks and hosted platform tests have recorded evidence. See the [October 21 completion report](docs/parity/db24832-completion-report-2026-10-21.html) and [verification ledger](modern/docs/verification.md) for exact revisions and observation limits. This work remains separate from `master`; a modern application release has not been published.

## Current capabilities

- DB2-style DNA parsing, symbolic sysvars, conditions, actions, mutation, reproduction, and species import/export.
- CPU and GPU simulation backends with runtime selection and automatic CPU fallback.
- Persistent shots, ties, collisions, corpses, chloroplast-based vegetation, toroidal worlds, obstacles, and teleporters.
- Starter-bot, Zerobot, and Zerobot-with-vegetables setup modes with per-species counts and mutation rates.
- Live speed, physics, sensing, energy, vegetation, mutation, and environment controls.
- Manual organism selection, dragging, movement, cloning, reproduction, following, killing, DNA editing, and bot export.
- Versioned modern saves, manual save/load, and autosave recovery.
- DB2-style organism sizing, inherited lineage colors and skins, heading markers, and selected-organism vision display.
- Immutable snapshots, stable `slot:generation` organism identities, and instanced rendering data suitable for large populations.

## Compatibility

Plain-text Darwinbots robot DNA remains a supported legacy interchange format. The modern engine resolves all 255 active DB2 2.48 sysvar names to their VB6 addresses and imports the bot corpus shipped in `Installer/bots`.

Modern saves use `DB3S` version 4, preserving native single-precision resource stores. Versions 2 and 3 still load; pending fractional carries are migrated into the stores once. Snapshot and C ABI version numbers remain unchanged. Native `.set` and `.gset` settings interchange and DB2 2.48.32 `.sim`/`.dbo` import/export are implemented. The recorded checks include genuine original simulations, original continuation of a port export, native organism interchange and modern save/resume. These finite fixtures do not establish compatibility with every historical file or version; see the [compatibility evidence and limits](modern/docs/verification.md#compatibility-and-observation-limits).

Internet Mode is excluded from the agreed scope; its three menu actions remain disabled. Offline exchange and local league/mode workflows are included. The original ActiveX/VB6 runtime infrastructure is not required by the modern application.

The VB6 source under `Darwinbots2/` is the behavioral authority when the modern engine and DarwinbotsC disagree. Compatibility means comparable user-facing behavior, not identical random sequences or cycle-for-cycle reproduction of DB2.

## Run the desktop application

Requirements:

- [.NET 10 SDK](https://dotnet.microsoft.com/download/dotnet/10.0)
- Rust 1.96 or newer
- A graphics driver supported by `wgpu`; GPU-less and unsupported systems can use the CPU backend

From the repository root on the completion branch:

```powershell
dotnet run --project modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj
```

The desktop project builds and copies the native Rust engine automatically.

On the Windows qualification machine, an already-built application is available in the original completion worktree at `artifacts/win-x64-native-ui66-final/Darwinbots.Desktop.exe`. It uses the .NET 10 runtime. That generated folder is local qualification output, not a committed package or published download; the [ledger](modern/docs/verification.md#existing-windows-package) records its identity.

To inspect the headless runner:

```powershell
cargo run -p darwinbots-cli -- --help
```

## Build and verification

```powershell
cargo test --workspace
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj
pwsh docs/verification/control-surface-audit/run-rendered-audit.ps1
```

Publish a framework-dependent Windows folder with:

```powershell
dotnet publish modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj `
  -c Release -r win-x64 --self-contained false -o modern/dist/win-x64
```

See the [verification ledger](modern/docs/verification.md) for measured results, exact commit qualifications, remaining observation limits and performance receipts. Run relevant checks serially; hosted CI does not replace physical GPU or native desktop evidence.

## Repository map

| Path | Purpose |
| --- | --- |
| `modern/engine/` | Headless Rust engine, C ABI, CPU backend, and `wgpu` acceleration |
| `modern/cli/` | Headless command-line runner |
| `modern/desktop/` | .NET 10/Avalonia desktop application and tests |
| `Installer/bots/` | Bundled legacy DNA corpus and starter species |
| `Darwinbots2/` | Authoritative VB6 reference implementation |
| `DarwinbotsC/` | Secondary C++ reference implementation |
| `docs/` | Current verification, parity evidence, and historical design records |

Start with the [documentation index](docs/README.md). The historical Darwinbots Wiki remains available at [wiki.darwinbots.com](http://wiki.darwinbots.com/w/Main_Page).

## License

Darwinbots Modern is distributed under the legacy DarwinBots terms in [LICENSE.md](LICENSE.md). Those terms require attribution and restrict redistribution to non-commercial, nonprofit distributions unless the author agrees otherwise.
