# Darwinbots Modern

Darwinbots Modern is a cross-platform artificial-life simulator inspired by Darwinbots 2. Robots execute compatible plain-text DNA, sense and interact with a shared world, reproduce, mutate, and compete for energy while the simulation runs as quickly as the selected hardware allows.

The modern application combines a Rust simulation engine, portable `wgpu` compute acceleration, a permanent CPU fallback, a headless CLI, and a .NET 10/Avalonia desktop interface. Windows 10 is the primary usability target; CI also builds and tests Linux x64, Intel macOS, and Apple silicon macOS packages.

> **Project status:** Playable and under active development. Existing modern controls are rendered and interaction-tested, but the project does not claim complete Darwinbots 2 feature parity or cycle-identical simulation results.

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

Plain-text Darwinbots robot DNA is the supported legacy interchange format. The modern engine resolves all 255 active DB2 2.48 sysvar names to their VB6 addresses and imports the bot corpus shipped in `Installer/bots`.

Modern saves use the versioned `DB3S` format. Legacy `.sim`, `.set`, `.gset`, `.dbo`, and `.stats` files are not supported. Online leagues and the original ActiveX/VB6 runtime infrastructure are outside the current product scope.

The VB6 source under `Darwinbots2/` is the behavioral authority when the modern engine and DarwinbotsC disagree. Compatibility means comparable user-facing behavior, not identical random sequences or cycle-for-cycle reproduction of DB2.

## Run the desktop application

Requirements:

- [.NET 10 SDK](https://dotnet.microsoft.com/download/dotnet/10.0)
- Rust 1.96 or newer
- A graphics driver supported by `wgpu`; GPU-less and unsupported systems can use the CPU backend

From the repository root:

```powershell
dotnet run --project modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj
```

The desktop project builds and copies the native Rust engine automatically.

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

See the [verification ledger](modern/docs/verification.md) for measured results, current acceptance gaps, and performance receipts.

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
