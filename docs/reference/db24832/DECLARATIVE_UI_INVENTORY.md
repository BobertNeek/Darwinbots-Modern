# Darwinbots 2.48.32 declarative UI inventory

This appendix inventories every form and declaratively defined control in the public 2.48.32 VB6 project. It is the mechanical companion to the interaction and visual-design specification. VB6 dimensions are in twips; at the conventional 96 DPI, 15 twips is approximately one pixel.

Generated from the 26 active `Form=` entries in [`Iersera.vbp`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Iersera.vbp) and their [source directory](https://github.com/darwinbots/Darwinbots2/tree/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2).

Runtime-created controls, captions changed in event code, bitmap content in `.frx` files, and common-dialog windows supplied by Windows are described in the authored UI spec, not invented here.

## DarwinBots (`MDIForm1`)

Source: [`MDIForm1.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/MDIForm1.frm)  
Class: `VB.MDIForm`. Client size: `14295 × 6495` twips. Border style: `default`. Startup position: `2`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Inet1` | `InetCtlsObjects.Inet` | — | L=2520, T=2160 | — |
| `unpause` | `VB.Timer` | — | L=4200, T=1560 | — |
| `ImageList1` | `MSComctlLib.ImageList` | — | L=2520, T=1440 | — |
| `Toolbar1` | `MSComctlLib.Toolbar` | — | L=0, T=0, W=14295, H=420 | TabIndex=1, Appearance=1 |
| `Toolbar1 / SunButton` | `VB.CheckBox` | ToolTipText: Toggles the Sun | L=10200, T=0, W=375, H=375 | Style=1, TabIndex=8 |
| `Toolbar1 / F1InternetButton` | `VB.CheckBox` | ToolTipText: Toggles Internet Mode | L=10680, T=0, W=375, H=375 | Style=1, TabIndex=7 |
| `Toolbar1 / ZoomLock` | `VB.CheckBox` | ToolTipText: Locks and unlocks being able to lock at areas outside of the arena. | L=9000, T=0, W=375, H=375 | Style=1, TabIndex=6 |
| `Toolbar1 / F1Piccy` | `VB.PictureBox` | — | L=11160, T=0, W=375, H=375 | TabIndex=5 |
| `Toolbar1 / Combo1` | `VB.ComboBox` | ToolTipText: Select the species for robot insertion | L=4800, T=25, W=1530, H=315 | Style=2, TabIndex=4 |
| `Toolbar1 / czo` | `VB.CommandButton` | ToolTipText: Zoom out | L=9720, T=0, W=375, H=375 | Style=1, TabIndex=3 |
| `Toolbar1 / czin` | `VB.CommandButton` | ToolTipText: Zoom in | L=9360, T=0, W=375, H=375 | Style=1, TabIndex=2, TabStop=0 |
| `StatusBar1` | `MSComctlLib.StatusBar` | — | L=0, T=6225, W=14295, H=270 | TabIndex=0 |
| `CommonDialog1` | `MSComDlg.CommonDialog` | — | L=1080, T=1440 | — |
| `file` | `VB.Menu` | Caption: File | — | Index=1 |
| `file / newsim` | `VB.Menu` | Caption: New Simulation | — | Index=2, Shortcut={F1} |
| `file / loadsim` | `VB.Menu` | Caption: Load Simulation | — | Index=3, Shortcut={F2} |
| `file / SaveSim` | `VB.Menu` | Caption: Save Simulation | — | Shortcut={F3} |
| `file / SaveSimWithoutMutations` | `VB.Menu` | Caption: Save Sim Without Mutations | — | Shortcut=^{F3} |
| `file / pause` | `VB.Menu` | Caption: Pause Sim (F12) | — | — |
| `file / p` | `VB.Menu` | Caption: - | — | Index=3 |
| `file / quit` | `VB.Menu` | Caption: Quit | — | Shortcut=^Q |
| `Edit` | `VB.Menu` | Caption: Edit | — | — |
| `Edit / inssp` | `VB.Menu` | Caption: Insert Organism... | — | Shortcut=^I |
| `Edit / sep11` | `VB.Menu` | Caption: - | — | — |
| `Edit / Species` | `VB.Menu` | Caption: Species... | — | Shortcut=^X |
| `Edit / fisica` | `VB.Menu` | Caption: General Settings... | — | Shortcut=^P |
| `Edit / costi` | `VB.Menu` | Caption: Physics and Costs... | — | Shortcut={F5} |
| `Edit / moltiplicatore` | `VB.Menu` | Caption: Global Mutation Rates... | — | Shortcut=^G |
| `Edit / Leagues` | `VB.Menu` | Caption: Restart and Leagues... | — | Shortcut=^F |
| `Edit / intOptionsOpen` | `VB.Menu` | Caption: Internet Options... | — | — |
| `Edit / sep90` | `VB.Menu` | Caption: - | — | — |
| `Edit / DisableFixing` | `VB.Menu` | Caption: Disallow robot from fixing or unfixing | — | — |
| `Edit / DisableArep` | `VB.Menu` | Caption: Disable asexual reproduction for non-repopulating robots | — | — |
| `Edit / sspp` | `VB.Menu` | Caption: - | — | — |
| `Edit / Autotag` | `VB.Menu` | Caption: Automatically tag by name... | — | — |
| `Edit / AutoFork` | `VB.Menu` | Caption: Enable Automatic Forking | — | — |
| `Edit / Sep80` | `VB.Menu` | Caption: - | — | — |
| `Edit / pbsett` | `VB.Menu` | Caption: Settings for PlayerBot Mode... | — | — |
| `Edit / pbOn` | `VB.Menu` | Caption: Enable PlayerBot Mode | — | Shortcut={F11} |
| `Edit / SepEYE` | `VB.Menu` | Caption: - | — | — |
| `Edit / showEyeDesign` | `VB.Menu` | Caption: Go to eye designer... | — | — |
| `Edit / Sep81` | `VB.Menu` | Caption: - | — | — |
| `Edit / RESOver` | `VB.Menu` | Caption: Restriction Overwrites | — | Shortcut={F8} |
| `Backgrounds` | `VB.Menu` | Caption: View | — | — |
| `Backgrounds / LoadPiccy` | `VB.Menu` | Caption: Import Background Picture | — | — |
| `Backgrounds / removepiccy` | `VB.Menu` | Caption: Remove Background Picture | — | — |
| `Backgrounds / sep987` | `VB.Menu` | Caption: - | — | — |
| `Backgrounds / ShowVisionGrid` | `VB.Menu` | Caption: Show Vision Grid | — | Checked=True |
| `Backgrounds / DisplayMovementVectors` | `VB.Menu` | Caption: Display Movement Vectors | — | Checked=True |
| `Backgrounds / DisplayShotImpacts` | `VB.Menu` | Caption: Display Shot Impacts | — | Checked=True |
| `Backgrounds / DisplayResourceGuages` | `VB.Menu` | Caption: Display Resource Guages | — | Checked=True |
| `Backgrounds / sep98` | `VB.Menu` | Caption: - | — | — |
| `Backgrounds / MonitorSettings` | `VB.Menu` | Caption: Settings for RGB Memory Monitor... | — | — |
| `Backgrounds / MonitorOn` | `VB.Menu` | Caption: RGB Memory Monitor | — | — |
| `popup` | `VB.Menu` | Caption: Robot | — | — |
| `popup / robinf` | `VB.Menu` | Caption: Show Robot Info | — | Shortcut=^R |
| `popup / par` | `VB.Menu` | Caption: Show Philogeny | — | Shortcut=^T |
| `popup / mutrat` | `VB.Menu` | Caption: Mutation Rates | — | Shortcut=^M |
| `popup / col` | `VB.Menu` | Caption: Change Color | — | Shortcut=^C |
| `popup / Sep` | `VB.Menu` | Caption: - | — | Index=14 |
| `popup / cons` | `VB.Menu` | Caption: Open Console | — | Shortcut=^Y |
| `popup / genact` | `VB.Menu` | Caption: View Gene Activations | — | Shortcut=^Z |
| `popup / sep3` | `VB.Menu` | Caption: - | — | — |
| `popup / ucci` | `VB.Menu` | Caption: Kill Robot | — | Shortcut=^K |
| `popup / sdna` | `VB.Menu` | Caption: Save Robot's DNA | — | Shortcut=^S |
| `popup / makenewspecies` | `VB.Menu` | Caption: Make New Species | — | — |
| `popup / sep4` | `VB.Menu` | Caption: - | — | — |
| `popup / selorg` | `VB.Menu` | Caption: Select Entire Organism | — | Shortcut=^{F1} |
| `popup / saveorg` | `VB.Menu` | Caption: Save Entire Organism | — | Shortcut=^{F2} |
| `popup / killorg` | `VB.Menu` | Caption: Kill Entire Organism | — | Shortcut=^E |
| `popup / sep99` | `VB.Menu` | Caption: - | — | — |
| `popup / fittest` | `VB.Menu` | Caption: Find Best | — | Shortcut=^B |
| `Objects` | `VB.Menu` | Caption: Objects | — | — |
| `Objects / ShotsMenu` | `VB.Menu` | Caption: Shots | — | — |
| `Objects / ShotsMenu / DontDecayNrgShots` | `VB.Menu` | Caption: Don't decay nrg shots | — | Checked=True |
| `Objects / ShotsMenu / DontDecayWstShots` | `VB.Menu` | Caption: Don't decay waste shots | — | Checked=True |
| `Objects / TiesMenu` | `VB.Menu` | Caption: Ties | — | — |
| `Objects / TiesMenu / DisableTies` | `VB.Menu` | Caption: Disable Ties | — | Checked=True |
| `Objects / TeleporterMenu` | `VB.Menu` | Caption: Teleporters | — | — |
| `Objects / TeleporterMenu / NewTeleportMenu` | `VB.Menu` | Caption: New Teleporter... | — | — |
| `Objects / TeleporterMenu / HighLightTeleportersMenu` | `VB.Menu` | Caption: Hilight Teleporters | — | Checked=True |
| `Objects / TeleporterMenu / DeleteTeleporterMenu` | `VB.Menu` | Caption: Delete Teleporter | — | — |
| `Objects / TeleporterMenu / DeleteTeleportersMenu` | `VB.Menu` | Caption: Delete All Teleporters | — | — |
| `Objects / sep47` | `VB.Menu` | Caption: Shapes | — | — |
| `Objects / sep47 / shapes` | `VB.Menu` | Caption: New Shape... | — | — |
| `Objects / sep47 / AddTenObstacles` | `VB.Menu` | Caption: Add Ten Random Shapes | — | — |
| `Objects / sep47 / DeleteTenObstacles` | `VB.Menu` | Caption: Delete 10 Random Shapes | — | — |
| `Objects / sep47 / DeleteShape` | `VB.Menu` | Caption: Delete Shape | — | — |
| `Objects / sep47 / DeleteAllShapes` | `VB.Menu` | Caption: Delete All Shapes | — | — |
| `Objects / MakeMaze` | `VB.Menu` | Caption: Mazes | — | — |
| `Objects / MakeMaze / HorizontalMaze` | `VB.Menu` | Caption: Simple Horizontal Maze | — | — |
| `Objects / MakeMaze / VerticaMaze` | `VB.Menu` | Caption: Simple Vertical Maze | — | — |
| `Objects / MakeMaze / DrawSpiral` | `VB.Menu` | Caption: Sprial | — | — |
| `Objects / MakeMaze / CheckerMaze` | `VB.Menu` | Caption: Checkerboard | — | — |
| `Objects / MakeMaze / PolarIce` | `VB.Menu` | Caption: Polar Ice | — | — |
| `Objects / MakeMaze / TrashCompactor` | `VB.Menu` | Caption: Trash Compactor | — | — |
| `EGrid` | `VB.Menu` | Caption: E-Grid | — | Index=45, Visible=0 |
| `EGrid / EGridEnabled` | `VB.Menu` | Caption: Enable | — | Checked=True |
| `EGrid / GridSize` | `VB.Menu` | Caption: Grid Size | — | — |
| `EGrid / GridSize / EGridLarge` | `VB.Menu` | Caption: Large | — | Checked=True |
| `EGrid / GridSize / EGridMedium` | `VB.Menu` | Caption: Medium | — | Checked=True |
| `EGrid / GridSize / EGridSmall` | `VB.Menu` | Caption: Small | — | Checked=True |
| `interent` | `VB.Menu` | Caption: Internet | — | — |
| `interent / F1Internet` | `VB.Menu` | Caption: Enabled | — | — |
| `interent / EditIntTeleporter` | `VB.Menu` | Caption: Internet Teleporter... | — | Enabled=0 |
| `recording` | `VB.Menu` | Caption: Recording | — | — |
| `recording / SafemodeBkp` | `VB.Menu` | Caption: Safemode Backup | — | — |
| `recording / SnpLiving` | `VB.Menu` | Caption: Snapshot of the living | — | — |
| `recording / SnpDead` | `VB.Menu` | Caption: Snapshot of the dead | — | — |
| `recording / SnpDead / SnpDeadEnable` | `VB.Menu` | Caption: Enable recording | — | — |
| `recording / SnpDead / SnpDeadExRep` | `VB.Menu` | Caption: Exclude repopulating robots | — | — |
| `help` | `VB.Menu` | Caption: Help | — | Index=5 |
| `help / DNAexp` | `VB.Menu` | Caption: DNA Help | — | Shortcut=^D |
| `help / sep102` | `VB.Menu` | Caption: - | — | — |
| `help / RobTagInfo` | `VB.Menu` | Caption: Robot Tag Information | — | — |
| `help / y_info` | `VB.Menu` | Caption: Survival Info | — | Visible=0 |
| `help / sep101` | `VB.Menu` | Caption: - | — | — |
| `help / about` | `VB.Menu` | Caption: About | — | Shortcut=^A |
| `TrayIconPopup` | `VB.Menu` | Caption: TrayIconPopup | — | Visible=0 |
| `TrayIconPopup / ShowInfo` | `VB.Menu` | Caption: Show Information | — | — |
| `TrayIconPopup / ShowInfo / CyclesNumber` | `VB.Menu` | Caption: CyclesNumber | — | — |
| `TrayIconPopup / ShowInfo / MutationValue` | `VB.Menu` | Caption: MutationValue | — | — |
| `TrayIconPopup / ShowInfo / TotalBots` | `VB.Menu` | Caption: TotalBots | — | — |
| `TrayIconPopup / ShowInfo / NumberBots` | `VB.Menu` | Caption: NumberBots | — | — |
| `TrayIconPopup / ShowInfo / NumberVeg` | `VB.Menu` | Caption: NumberVeg | — | — |
| `TrayIconPopup / ShowInfo / BotsBorn` | `VB.Menu` | Caption: BotsBorn | — | — |
| `TrayIconPopup / ShowDB` | `VB.Menu` | Caption: Show DarwinBots | — | — |

## Dati del robot (`datirob`)

Source: [`robdata.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/robdata.frm)  
Class: `VB.Form`. Client size: `10110 × 7545` twips. Border style: `4`. Startup position: `default`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `ShrinkWin` | `VB.CommandButton` | Caption: Close details | L=3480, T=7150, W=1770, H=240 | TabIndex=28 |
| `Frame2` | `VB.Frame` | — | L=3240, T=0, W=6795, H=7515 | TabIndex=24 |
| `Frame2 / btnMark` | `VB.CommandButton` | Caption: Mark a location | L=2160, T=7150, W=1770, H=240 | TabIndex=59 |
| `Frame2 / MemoryStateCheck` | `VB.CheckBox` | Caption: Display memory post DNA execution but before cycle executes | L=2160, T=7080, W=5775, H=375 | TabIndex=58 |
| `Frame2 / dnatext` | `RichTextLib.RichTextBox` | — | L=0, T=360, W=6555, H=6705 | TabIndex=27, ScrollBars=2 |
| `Frame2 / robtag` | `VB.Label` | ToolTipText: Double click to edit | L=40, T=120, W=6615, H=255 | TabIndex=62 |
| `Frame1` | `VB.Frame` | — | L=120, T=0, W=3075, H=7515 | TabIndex=0 |
| `Frame1 / MemoryCommand` | `VB.CommandButton` | Caption: Memory -> | L=1605, T=6360, W=1365, H=285 | TabIndex=53 |
| `Frame1 / Command1` | `VB.CommandButton` | Caption: Discendenti | L=90, T=6000, W=1410, H=300 | TabIndex=46 |
| `Frame1 / dnashow` | `VB.CommandButton` | Caption: Dna -> | L=1605, T=6720, W=1365, H=285 | TabIndex=45 |
| `Frame1 / MutDetails` | `VB.CommandButton` | Caption: Mutation details-> | L=1605, T=7080, W=1365, H=285 | TabIndex=44 |
| `Frame1 / Command2` | `VB.CommandButton` | Caption: Show activations | L=90, T=6720, W=1425, H=285 | Index=0, TabIndex=43 |
| `Frame1 / Command3` | `VB.CommandButton` | Caption: Open console | L=90, T=6360, W=1425, H=285 | TabIndex=42 |
| `Frame1 / Repro` | `VB.CommandButton` | Caption: Reproduce | L=90, T=7080, W=1425, H=285 | Index=1, TabIndex=41 |
| `Frame1 / Line1` | `VB.Line` | — | — | — |
| `Frame1 / Label11` | `VB.Label` | Caption: Chloroplasts | L=120, T=4800, W=1215, H=195 | TabIndex=61 |
| `Frame1 / ChlrLabel` | `VB.Label` | Caption: VTimer | L=2100, T=4800, W=795, H=195 | TabIndex=60, Alignment=1 |
| `Frame1 / Label10` | `VB.Label` | Caption: Radius | L=120, T=2880, W=975, H=255 | TabIndex=57 |
| `Frame1 / RadiusLabel` | `VB.Label` | Caption: radius | L=2100, T=2880, W=795, H=255 | TabIndex=56, Alignment=1 |
| `Frame1 / Label9` | `VB.Label` | Caption: Velocity | L=120, T=3120, W=1215, H=195 | TabIndex=55 |
| `Frame1 / VelocityLabel` | `VB.Label` | Caption: velocity | L=2100, T=3120, W=795, H=195 | TabIndex=54, Alignment=1 |
| `Frame1 / VTimerLabel` | `VB.Label` | Caption: VTimer | L=2100, T=4560, W=795, H=195 | TabIndex=52, Alignment=1 |
| `Frame1 / PoisonLabel` | `VB.Label` | Caption: poison | L=2100, T=4320, W=795, H=195 | TabIndex=51, Alignment=1 |
| `Frame1 / Label7` | `VB.Label` | Caption: Poison | L=120, T=4320, W=1215, H=195 | TabIndex=50 |
| `Frame1 / Label6` | `VB.Label` | Caption: Virus Timer | L=120, T=4560, W=1215, H=195 | TabIndex=49 |
| `Frame1 / UniqueBotID` | `VB.Label` | Caption: unique id | L=2100, T=960, W=795, H=225 | TabIndex=48, Alignment=1 |
| `Frame1 / UniqueIDLabel` | `VB.Label` | Caption: Unique ID; ToolTipText: The unique ID of this bot.  No other bot will ever have this number in the sim. | L=105, T=960, W=1530, H=195 | TabIndex=47 |
| `Frame1 / robslime` | `VB.Label` | Caption: slime | L=2100, T=4080, W=795, H=195 | TabIndex=40, Alignment=1 |
| `Frame1 / Label5` | `VB.Label` | Caption: Slime | L=120, T=4080, W=1215, H=195 | TabIndex=39 |
| `Frame1 / robshell` | `VB.Label` | Caption: shell | L=2100, T=3840, W=795, H=195 | TabIndex=38, Alignment=1 |
| `Frame1 / Label4` | `VB.Label` | Caption: Shell | L=120, T=3840, W=1215, H=195 | TabIndex=37 |
| `Frame1 / robvenom` | `VB.Label` | Caption: venom | L=2100, T=3600, W=795, H=195 | TabIndex=36, Alignment=1 |
| `Frame1 / Label3` | `VB.Label` | Caption: Venom | L=120, T=3600, W=1215, H=195 | TabIndex=35 |
| `Frame1 / robmass` | `VB.Label` | Caption: mass | L=2100, T=2640, W=795, H=255 | TabIndex=34, Alignment=1 |
| `Frame1 / MassLabel` | `VB.Label` | Caption: Mass | L=120, T=2640, W=975, H=255 | TabIndex=33 |
| `Frame1 / robbody` | `VB.Label` | Caption: body | L=2100, T=2400, W=795, H=255 | TabIndex=32, Alignment=1 |
| `Frame1 / BodyLabel` | `VB.Label` | Caption: Body | L=105, T=2400, W=735, H=255 | TabIndex=31 |
| `Frame1 / wasteval` | `VB.Label` | Caption: waste | L=2100, T=3360, W=795, H=180 | TabIndex=30, Alignment=1 |
| `Frame1 / Waste` | `VB.Label` | Caption: Waste | L=120, T=3360, W=1455, H=195 | TabIndex=29 |
| `Frame1 / LastOwnLab` | `VB.Label` | Caption: lastownername | L=1140, T=480, W=1845, H=225 | TabIndex=26 |
| `Frame1 / Label1` | `VB.Label` | Caption: Last Sim: | L=105, T=480, W=810, H=195 | TabIndex=25 |
| `Frame1 / totlenlab` | `VB.Label` | Caption: dnalen | L=2100, T=5520, W=795, H=195 | TabIndex=23, Alignment=1 |
| `Frame1 / totlen` | `VB.Label` | Caption: Lunghezza DNA | L=120, T=5520, W=1485, H=240 | TabIndex=22 |
| `Frame1 / robgener` | `VB.Label` | Caption: gener | L=2100, T=1680, W=795, H=240 | TabIndex=21, Alignment=1 |
| `Frame1 / generation` | `VB.Label` | Caption: Generation | L=105, T=1680, W=1065, H=195 | TabIndex=20 |
| `Frame1 / robgene` | `VB.Label` | Caption: gennum | L=2100, T=5760, W=795, H=195 | TabIndex=19, Alignment=1 |
| `Frame1 / robover` | `VB.Label` | Caption: overallm | L=2100, T=5280, W=795, H=195 | TabIndex=18, Alignment=1 |
| `Frame1 / rgenes` | `VB.Label` | Caption: Numero di geni | L=105, T=5760, W=1485, H=240 | TabIndex=17 |
| `Frame1 / roverall` | `VB.Label` | Caption: Mutazioni totali | L=105, T=5280, W=1590, H=195 | TabIndex=16 |
| `Frame1 / robmutations` | `VB.Label` | Caption: mut | L=2100, T=5040, W=795, H=195 | TabIndex=15, Alignment=1 |
| `Frame1 / rmutation` | `VB.Label` | Caption: Mutazioni nuove | L=120, T=5040, W=2010, H=240 | TabIndex=14 |
| `Frame1 / rfname` | `VB.Label` | Caption: Species: | L=120, T=240, W=765, H=195 | TabIndex=13 |
| `Frame1 / robson` | `VB.Label` | Caption: sonnum | L=2100, T=1920, W=795, H=180 | TabIndex=12, Alignment=1 |
| `Frame1 / rson` | `VB.Label` | Caption: Offspring | L=105, T=1920, W=1905, H=195 | TabIndex=11 |
| `Frame1 / robage` | `VB.Label` | Caption: age | L=2100, T=1440, W=795, H=225 | TabIndex=10, Alignment=1 |
| `Frame1 / robparent` | `VB.Label` | Caption: parent id | L=2100, T=1200, W=795, H=225 | TabIndex=9, Alignment=1 |
| `Frame1 / robnrg` | `VB.Label` | Caption: nrg | L=2100, T=2160, W=795, H=225 | TabIndex=8, Alignment=1 |
| `Frame1 / robnum` | `VB.Label` | Caption: rob array id | L=2100, T=720, W=795, H=225 | TabIndex=7, Alignment=1 |
| `Frame1 / rage` | `VB.Label` | Caption: Age (cycles) | L=105, T=1440, W=1905, H=240 | TabIndex=6 |
| `Frame1 / rparent` | `VB.Label` | Caption: Id del genitore | L=105, T=1200, W=1170, H=195 | TabIndex=5 |
| `Frame1 / rnrg` | `VB.Label` | Caption: Energy | L=105, T=2160, W=645, H=240 | TabIndex=4 |
| `Frame1 / Rnum` | `VB.Label` | Caption: Robot array ID | L=120, T=720, W=1170, H=195 | TabIndex=3 |
| `Frame1 / robfname` | `VB.Label` | Caption: fname; ToolTipText: Double click to copy | L=1140, T=240, W=1860, H=225 | TabIndex=2 |
| `Frame1 / Label2` | `VB.Label` | Caption: ------ | L=2100, T=6000, W=795, H=240 | TabIndex=1, Alignment=1 |

## Darwinbots (`InfoForm`)

Source: [`InfoForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/InfoForm.frm)  
Class: `VB.Form`. Client size: `9615 × 6000` twips. Border style: `1`. Startup position: `1`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Command1` | `VB.CommandButton` | Caption: &OK | L=7680, T=5520, W=1815, H=375 | Style=1, TabIndex=8, Appearance=0 |
| `Label8` | `VB.Label` | Caption: $"InfoForm.frx":912B | L=5070, T=4785, W=2295, H=1035 | BorderStyle=1, TabIndex=7, Appearance=0 |
| `Label7` | `VB.Label` | Caption: $"InfoForm.frx":91C5 | L=6840, T=15, W=2730, H=1320 | TabIndex=6 |
| `Line6` | `VB.Line` | — | — | — |
| `Line5` | `VB.Line` | — | — | — |
| `Line4` | `VB.Line` | — | — | — |
| `Line3` | `VB.Line` | — | — | — |
| `Line2` | `VB.Line` | — | — | — |
| `Line1` | `VB.Line` | — | — | — |
| `Label6` | `VB.Label` | Caption: $"InfoForm.frx":92B1 | L=4845, T=1500, W=2175, H=1275 | TabIndex=5 |
| `Label5` | `VB.Label` | Caption: Attraverso i legami permanenti si possono spedire informazioni ed energia.  | L=4770, T=3210, W=2205, H=885 | TabIndex=4 |
| `Label4` | `VB.Label` | Caption: $"InfoForm.frx":9357 | L=1305, T=4830, W=2520, H=1110 | TabIndex=3 |
| `Label3` | `VB.Label` | Caption: $"InfoForm.frx":93FC | L=1230, T=3270, W=2730, H=1335 | TabIndex=2 |
| `Label2` | `VB.Label` | Caption: $"InfoForm.frx":94FB | L=1230, T=1485, W=3435, H=1650 | TabIndex=1 |
| `Label1` | `VB.Label` | Caption: $"InfoForm.frx":9699 | L=4200, T=15, W=2550, H=1320 | TabIndex=0 |

## Custom Color (`ColorForm`)

Source: [`colorform.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/colorform.frm)  
Class: `VB.Form`. Client size: `3945 × 2235` twips. Border style: `1`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `btnWrite` | `VB.CommandButton` | Caption: Write color as data into DNA file | L=1080, T=1200, W=2415, H=375 | TabIndex=8 |
| `UseColor` | `VB.CommandButton` | Caption: Use This Color | L=1080, T=1680, W=2415, H=375 | TabIndex=7 |
| `SliderR` | `MSComctlLib.Slider` | — | L=630, T=420, W=2850, H=225 | TabIndex=4 |
| `SliderG` | `MSComctlLib.Slider` | — | L=630, T=630, W=2850, H=225 | TabIndex=5 |
| `SliderB` | `MSComctlLib.Slider` | — | L=630, T=840, W=2850, H=225 | TabIndex=6 |
| `LabelB` | `VB.Label` | Caption: 0 | L=15, T=780, W=450, H=195 | TabIndex=3, Alignment=1 |
| `LabelG` | `VB.Label` | Caption: 0 | L=15, T=585, W=450, H=195 | TabIndex=2, Alignment=1 |
| `LabelR` | `VB.Label` | Caption: 0 | L=15, T=390, W=450, H=195 | TabIndex=1, Alignment=1 |
| `Label1` | `VB.Label` | Caption: R G  B | L=3615, T=405, W=195, H=615 | TabIndex=0 |
| `Shape1` | `VB.Shape` | — | L=120, T=1320, W=615, H=690 | — |

## Parentele (`parentele`)

Source: [`parentele.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/parentele.frm)  
Class: `VB.Form`. Client size: `5220 × 2790` twips. Border style: `1`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Frame2` | `VB.Frame` | — | L=60, T=0, W=5100, H=2715 | TabIndex=0 |
| `Frame2 / Command2` | `VB.CommandButton` | Caption: Mostra tutta la linea di parentele | L=1155, T=1575, W=2535, H=375 | TabIndex=7 |
| `Frame2 / Command1` | `VB.CommandButton` | Caption: Evidenzia discendenti | L=1365, T=735, W=2070, H=375 | TabIndex=6 |
| `Frame2 / Text1` | `VB.TextBox` | Text: 4 | L=3045, T=300, W=285, H=195 | BorderStyle=0, TabIndex=3, Alignment=1 |
| `Frame2 / Command4` | `VB.CommandButton` | Caption: calcola | L=1890, T=255, W=975, H=315 | TabIndex=1 |
| `Frame2 / UpDown1` | `ComCtl2.UpDown` | — | L=3435, T=270, W=240, H=270 | Enabled=True, Value=1, TabIndex=2 |
| `Frame2 / Line4` | `VB.Line` | — | — | — |
| `Frame2 / Line3` | `VB.Line` | — | — | — |
| `Frame2 / Label5` | `VB.Label` | Caption: figlio | L=3045, T=2310, W=435, H=225 | TabIndex=10 |
| `Frame2 / Label4` | `VB.Label` | Caption: genitore | L=1365, T=2310, W=645, H=225 | TabIndex=9 |
| `Frame2 / Line2` | `VB.Line` | — | — | — |
| `Frame2 / Line1` | `VB.Line` | — | — | — |
| `Frame2 / Shape2` | `VB.Shape` | — | L=3885, T=2100, W=225, H=225 | — |
| `Frame2 / Shape1` | `VB.Shape` | — | L=840, T=2100, W=225, H=225 | — |
| `Frame2 / Label3` | `VB.Label` | Caption: 0 | L=1320, T=300, W=555, H=255 | TabIndex=8 |
| `Frame2 / Label1` | `VB.Label` | Caption: Discendenti : | L=270, T=300, W=930, H=255 | TabIndex=5 |
| `Frame2 / Label2` | `VB.Label` | Caption: generazioni | L=3780, T=285, W=975, H=255 | TabIndex=4 |

## Form1 (`Consoleform`)

Source: [`console.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/console.frm)  
Class: `VB.Form`. Client size: `4710 × 2550` twips. Border style: `4`. Startup position: `default`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Text1` | `RichTextLib.RichTextBox` | — | L=0, T=240, W=4695, H=2025 | TabIndex=10, ScrollBars=2 |
| `Text2` | `VB.TextBox` | — | L=0, T=2280, W=4695, H=285 | TabIndex=1 |
| `Picture1` | `VB.PictureBox` | — | L=0, T=-60, W=4695, H=315 | BorderStyle=0, TabIndex=0, TabStop=0 |
| `Picture1 / ClearButton` | `VB.CommandButton` | — | L=3885, T=45, W=375, H=300 | Style=1, TabIndex=9 |
| `Picture1 / Command1` | `VB.CommandButton` | — | L=4245, T=45, W=375, H=300 | Style=1, TabIndex=6 |
| `Picture1 / eyebut` | `VB.CommandButton` | — | L=1575, T=45, W=375, H=300 | Style=1, TabIndex=5 |
| `Picture1 / playbut` | `VB.CommandButton` | — | L=0, T=45, W=375, H=300 | Style=1, TabIndex=2 |
| `Picture1 / Command3` | `VB.CommandButton` | — | L=1935, T=30, W=375, H=300 | Style=1, TabIndex=8 |
| `Picture1 / Command2` | `VB.CommandButton` | — | L=2280, T=45, W=375, H=300 | Style=1, TabIndex=7 |
| `Picture1 / debug` | `VB.CommandButton` | — | L=2650, T=44, W=375, H=300 | Style=1, TabIndex=11 |
| `Picture1 / pausebut` | `VB.CommandButton` | — | L=360, T=30, W=375, H=300 | Style=1, TabIndex=3 |
| `Picture1 / cyclebut` | `VB.CommandButton` | — | L=720, T=30, W=375, H=300 | Style=1, TabIndex=4 |

## Le informazioni di cui non potete fare a meno! (`frmAbout`)

Source: [`frmAbout.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmAbout.frm)  
Class: `VB.Form`. Client size: `6825 × 4455` twips. Border style: `1`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Text1` | `VB.TextBox` | Text: "frmAbout.frx":058A | L=600, T=1920, W=5775, H=1455 | TabIndex=7, ScrollBars=2 |
| `cmdOK` | `VB.CommandButton` | Caption: Mi sento meglio | L=4800, T=3960, W=1500, H=345 | TabIndex=0, Default=True, Cancel=True |
| `Label5` | `VB.Label` | Caption: Post 2.45.1 revisions by Botsareus (hit me up on twitter CosmoTwitt007) | L=-360, T=1680, W=6735, H=255 | TabIndex=10, Alignment=1 |
| `Label4` | `VB.Label` | Caption: Further revisions by EricL (eric@sulaadventures.com) | L=1560, T=1440, W=4815, H=255 | TabIndex=9, Alignment=1 |
| `Label3` | `VB.Label` | Caption: Revisions by Purple Youko (Higginsb@missouri.edu) | L=1800, T=1200, W=4575, H=255 | TabIndex=8, Alignment=1 |
| `lblTitle` | `VB.Label` | Caption: DarwinBots | L=1920, T=120, W=3015, H=660 | TabIndex=6 |
| `lblDescription` | `VB.Label` | Caption: Carlo Comis   (comis@libero.it) | L=3720, T=960, W=2775, H=210 | TabIndex=5 |
| `Label1` | `VB.Label` | Caption: Robottini genetici! | L=2880, T=735, W=1485, H=255 | TabIndex=4 |
| `Line1` | `VB.Line` | — | — | — |
| `Label2` | `VB.Label` | Caption: Se vorrete segnalarmi un bug o un commento o qualsiasi altra cosa ve ne saro' grato!              E-mail: comis@libero.it  | L=600, T=3480, W=3060, H=540 | TabIndex=3 |
| `Label8` | `VB.Label` | Caption: 2002-2003 Carlo Comis | L=120, T=4080, W=2370, H=225 | TabIndex=2 |
| `Label6` | `VB.Label` | Caption: Version 2.44.1 - Dec 2008 | L=120, T=960, W=2505, H=225 | TabIndex=1, Alignment=2 |

## Simulation Settings (`optionsform`)

Source: [`OptionsForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/OptionsForm.frm)  
Class: `VB.Form`. Client size: `10560 × 6765` twips. Border style: `4`. Startup position: `2`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Frame5` | `VB.Frame` | — | L=240, T=5760, W=9660, H=645 | TabIndex=0 |
| `Frame5 / CommonDialog1` | `MSComDlg.CommonDialog` | — | L=5760, T=120 | — |
| `Frame5 / gset` | `VB.CommandButton` | Caption: Global settings | L=4440, T=225, W=1305, H=330 | TabIndex=196 |
| `Frame5 / OKButton` | `VB.CommandButton` | Caption: Change | L=8520, T=225, W=975, H=330 | TabIndex=4 |
| `Frame5 / StartNew` | `VB.CommandButton` | Caption: Start new | L=7440, T=225, W=960, H=330 | TabIndex=3 |
| `Frame5 / SaveSettings` | `VB.CommandButton` | Caption: Save settings | L=1560, T=225, W=1305, H=330 | TabIndex=2 |
| `Frame5 / LoadSettings` | `VB.CommandButton` | Caption: Load settings | L=210, T=225, W=1305, H=330 | TabIndex=1 |
| `SSTab1` | `TabDlg.SSTab` | — | L=0, T=0, W=10500, H=6690 | Style=1, TabIndex=5 |
| `SSTab1 / btnSetSB` | `VB.CommandButton` | Caption: Set SB settings | L=-70440, T=3120, W=2055, H=375 | TabIndex=202 |
| `SSTab1 / btnSetF2` | `VB.CommandButton` | Caption: Set F2/F3/MB settings | L=-72600, T=3120, W=2055, H=375 | TabIndex=200 |
| `SSTab1 / btnSetF1` | `VB.CommandButton` | Caption: Set F1/SS settings | L=-74760, T=3120, W=2055, H=375 | TabIndex=199 |
| `SSTab1 / NativeSpeciesButton` | `VB.CommandButton` | Caption: List Non-Native Species ; ToolTipText: Add a new robot type to the simulation | L=1440, T=3240, W=2295, H=375 | TabIndex=186 |
| `SSTab1 / DisableMutationsCheck` | `VB.CheckBox` | Caption: Disable All Mutations | L=-74640, T=840, W=1815, H=375 | TabIndex=185 |
| `SSTab1 / Frame20` | `VB.Frame` | Caption: Physics | L=-74820, T=420, W=4635, H=5235 | TabIndex=150 |
| `SSTab1 / Frame20 / ToPhysics` | `VB.CommandButton` | Caption: Custom Physics | L=1800, T=1680, W=1950, H=375 | TabIndex=158 |
| `SSTab1 / Frame20 / EfficiencyCombo` | `VB.ComboBox` | Text: Set Efficiency | L=2880, T=2400, W=1395, H=315 | TabIndex=157 |
| `SSTab1 / Frame20 / Frame26` | `VB.Frame` | Caption: The Big Blue Screen Acts Like A  | L=120, T=300, W=4395, H=1935 | TabIndex=154 |
| `SSTab1 / Frame20 / Frame26 / FluidSolidRadio` | `VB.OptionButton` | Caption: Custom | L=360, T=1440, W=3015, H=195 | Index=2, TabIndex=168 |
| `SSTab1 / Frame20 / Frame26 / FluidSolidRadio` | `VB.OptionButton` | Caption: Solid | L=360, T=960, W=975, H=195 | Index=1, TabIndex=167 |
| `SSTab1 / Frame20 / Frame26 / FluidSolidRadio` | `VB.OptionButton` | Caption: Fluid | L=360, T=480, W=975, H=195 | Index=0, TabIndex=166 |
| `SSTab1 / Frame20 / Frame26 / DragCombo` | `VB.ComboBox` | Text: Set Drag | L=1680, T=360, W=1995, H=315 | Enabled=0, TabIndex=156 |
| `SSTab1 / Frame20 / Frame26 / FrictionCombo` | `VB.ComboBox` | Text: Set Friction | L=1680, T=840, W=1995, H=315 | Enabled=0, TabIndex=155 |
| `SSTab1 / Frame20 / BrownianCombo` | `VB.ComboBox` | Text: Set Brownian | L=2880, T=2760, W=1395, H=315 | TabIndex=153 |
| `SSTab1 / Frame20 / GravityCombo` | `VB.ComboBox` | Text: Set Gravity | L=2880, T=3120, W=1395, H=315 | TabIndex=152 |
| `SSTab1 / Frame20 / MaxVelSlider` | `MSComctlLib.Slider` | ToolTipText: Maximum bot velocity | L=240, T=3840, W=4215, H=495 | TabIndex=151 |
| `SSTab1 / Frame20 / Elasticity` | `MSComctlLib.Slider` | ToolTipText: Controls how elastic bots act during collisions. | L=240, T=4680, W=4215, H=495 | TabIndex=172 |
| `SSTab1 / Frame20 / Label47` | `VB.Label` | Caption: Brownian Movment and Vertical Gravity currently overwritten by Tidal Mode | L=480, T=2760, W=2175, H=615 | TabIndex=210 |
| `SSTab1 / Frame20 / Label38` | `VB.Label` | Caption: Fast | L=4080, T=3600, W=375, H=255 | TabIndex=177 |
| `SSTab1 / Frame20 / Label37` | `VB.Label` | Caption: Slow | L=240, T=3600, W=615, H=255 | TabIndex=176 |
| `SSTab1 / Frame20 / Label33` | `VB.Label` | Caption: Marbles | L=3840, T=4440, W=615, H=255 | TabIndex=175 |
| `SSTab1 / Frame20 / Label18` | `VB.Label` | Caption: Ghosts | L=240, T=4440, W=615, H=255 | TabIndex=174 |
| `SSTab1 / Frame20 / Label11` | `VB.Label` | Caption: Collision Elasticity | L=1440, T=4440, W=1335, H=255 | TabIndex=173 |
| `SSTab1 / Frame20 / Label7` | `VB.Label` | Caption: Movement Efficiency | L=480, T=2400, W=1875, H=255 | Index=1, TabIndex=162 |
| `SSTab1 / Frame20 / Label7` | `VB.Label` | Caption: Brownian Movement | L=480, T=2760, W=1875, H=255 | Index=2, TabIndex=161 |
| `SSTab1 / Frame20 / Label7` | `VB.Label` | Caption: Vertical Gravity | L=480, T=3120, W=1875, H=255 | Index=3, TabIndex=160 |
| `SSTab1 / Frame20 / Label12` | `VB.Label` | Caption: Max Velocity | L=1920, T=3600, W=975, H=255 | TabIndex=159 |
| `SSTab1 / GenPropFrame` | `VB.Frame` | Caption: General Properties | L=-74880, T=360, W=9885, H=5295 | TabIndex=80 |
| `SSTab1 / GenPropFrame / Frame22` | `VB.Frame` | Caption: Wrap Around | L=120, T=2160, W=2055, H=795 | TabIndex=146 |
| `SSTab1 / GenPropFrame / Frame22 / TopDownCheck` | `VB.CheckBox` | Caption: Top / Down Wrap | L=120, T=240, W=1605, H=255 | TabIndex=148, Alignment=1 |
| `SSTab1 / GenPropFrame / Frame22 / RightLeftCheck` | `VB.CheckBox` | Caption: Left / Right Wrap | L=120, T=480, W=1605, H=255 | TabIndex=147, Alignment=1 |
| `SSTab1 / GenPropFrame / Frame23` | `VB.Frame` | Caption: Misc. Controls | L=120, T=4200, W=2055, H=975 | TabIndex=145 |
| `SSTab1 / GenPropFrame / Frame23 / FixBotRadius` | `VB.CheckBox` | Caption: Fix bot radii; ToolTipText: Shots will live forever unitl they impact a bot. | L=120, T=360, W=1695, H=255 | TabIndex=178, Alignment=1 |
| `SSTab1 / GenPropFrame / Frame24` | `VB.Frame` | Caption: Field Controls | L=120, T=240, W=4035, H=795 | TabIndex=138 |
| `SSTab1 / GenPropFrame / Frame24 / FieldSizeSlide` | `MSComctlLib.Slider` | — | L=60, T=480, W=2415, H=210 | Value=2, TabIndex=139 |
| `SSTab1 / GenPropFrame / Frame24 / Label28` | `VB.Label` | Caption: Width: | L=2640, T=240, W=495, H=240 | Index=0, TabIndex=144 |
| `SSTab1 / GenPropFrame / Frame24 / Label32` | `VB.Label` | Caption: Height: | L=2640, T=480, W=525, H=240 | TabIndex=143 |
| `SSTab1 / GenPropFrame / Frame24 / FWidthLab` | `VB.Label` | Caption: XXXX | L=3270, T=240, W=690, H=195 | TabIndex=142 |
| `SSTab1 / GenPropFrame / Frame24 / FHeightLab` | `VB.Label` | Caption: XXXX | L=3270, T=480, W=690, H=240 | TabIndex=141 |
| `SSTab1 / GenPropFrame / Frame24 / FieldSizeLab` | `VB.Label` | Caption: Size | L=1080, T=240, W=615, H=255 | TabIndex=140 |
| `SSTab1 / GenPropFrame / Frame25` | `VB.Frame` | Caption: Waste | L=120, T=3120, W=2055, H=615 | TabIndex=134 |
| `SSTab1 / GenPropFrame / Frame25 / CustomWaste` | `VB.TextBox` | Text: 400 | L=1020, T=240, W=555, H=285 | TabIndex=136 |
| `SSTab1 / GenPropFrame / Frame25 / WasteThresholdUpDown` | `ComCtl2.UpDown` | — | L=1680, T=240, W=255, H=285 | Enabled=True, TabIndex=135 |
| `SSTab1 / GenPropFrame / Frame25 / Label21` | `VB.Label` | Caption: Threshold | L=180, T=300, W=735, H=195 | TabIndex=137 |
| `SSTab1 / GenPropFrame / Frame27` | `VB.Frame` | Caption: Random Numbers | L=120, T=1200, W=2055, H=975 | TabIndex=131 |
| `SSTab1 / GenPropFrame / Frame27 / Newseed` | `VB.CommandButton` | Caption: Change seed | L=120, T=240, W=1815, H=255 | TabIndex=197 |
| `SSTab1 / GenPropFrame / Frame27 / UserSeedText` | `VB.TextBox` | Text: 1234 | L=1140, T=540, W=795, H=285 | TabIndex=132 |
| `SSTab1 / GenPropFrame / Frame27 / Label58` | `VB.Label` | Caption: Seed Value | L=165, T=600, W=915, H=195 | TabIndex=133 |
| `SSTab1 / GenPropFrame / VeggyControls` | `VB.Frame` | Caption: Veggy Controls | L=4200, T=180, W=5610, H=4995 | TabIndex=99 |
| `SSTab1 / GenPropFrame / VeggyControls / Energy` | `VB.CommandButton` | Caption: Energy Management | L=3960, T=1080, W=1215, H=570 | TabIndex=209 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16` | `VB.Frame` | Caption: Pond Mode | L=240, T=240, W=3375, H=1515 | TabIndex=120 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17` | `VB.Frame` | Caption: Light Demo | L=2100, T=180, W=1215, H=1275 | TabIndex=124 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / tmrLight` | `VB.Timer` | — | L=840, T=360 | Enabled=0 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / EnergyScalingFactor` | `VB.TextBox` | Text: 42; ToolTipText: Scale the brightness of the graph to the left.  Value sets the energy gain per cycle above which the brightness is set to maximum. | L=540, T=900, W=375, H=285 | Visible=0, TabIndex=125 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / Label23` | `VB.Label` | Caption: Energy Scaling Factor | L=540, T=240, W=555, H=615 | TabIndex=126 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=15, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=14, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=13, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=12, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=11, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=10, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=9, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=8, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=7, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=6, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=5, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=4, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=3, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=2, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=1, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / LightStrata` | `VB.Line` | — | — | Index=0, BorderStyle=6 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Frame17 / Shape4` | `VB.Shape` | — | L=90, T=255, W=425, H=915 | — |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Gradient` | `VB.TextBox` | Text: 0; ToolTipText: Set the gradient for light transmission through the water. A value of zero means no light reduction at any depth. | L=1350, T=780, W=420, H=285 | TabIndex=123 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / LightText` | `VB.TextBox` | Text: 100; ToolTipText: Set the light intensity to feed your veggies | L=1350, T=480, W=420, H=285 | TabIndex=122 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / Pondcheck` | `VB.CheckBox` | Caption: Pondmode; ToolTipText: $"OptionsForm.frx":01D0 | L=60, T=240, W=1980, H=255 | TabIndex=121, Alignment=1 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / GradientUpDn` | `ComCtl2.UpDown` | ToolTipText: Set the gradient for light transmission through the water. A value of zero means no light reduction at any depth. | L=1800, T=780, W=255, H=285 | Enabled=True, Value=1, TabIndex=127 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / LightUpDn` | `ComCtl2.UpDown` | ToolTipText: Set the light intensity to feed your veggies | L=1800, T=480, W=255, H=285 | Enabled=True, TabIndex=128 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / LightLabel` | `VB.Label` | Caption: Light intensity; ToolTipText: Set the light intensity to feed your veggies | L=90, T=540, W=975, H=240 | TabIndex=130 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame16 / GradientLabel` | `VB.Label` | Caption: Sediment Level; ToolTipText: Set the gradient for light transmission through the water. A value of zero means no light reduction at any depth. | L=90, T=840, W=1155, H=240 | TabIndex=129 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame31` | `VB.Frame` | Caption: Veg Body/NRG Distribution | L=1920, T=4020, W=3630, H=915 | TabIndex=116 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame31 / BodyNrgDist` | `MSComctlLib.Slider` | ToolTipText: When a veg gets NRG, what percentage should go into body points? | L=480, T=240, W=2355, H=570 | TabIndex=117, TabStop=0 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame31 / LabelZZ` | `VB.Label` | Caption: Body | L=2880, T=420, W=375, H=195 | Index=18, TabIndex=119 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame31 / LabelZZ` | `VB.Label` | Caption: NRG | L=120, T=420, W=375, H=195 | Index=17, TabIndex=118 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29` | `VB.Frame` | Caption: Rates for Repopulation | L=2040, T=1800, W=3375, H=2175 | TabIndex=103 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / KillDistVegsCheck` | `VB.CheckBox` | Caption: Force robots to sustain their own chloroplasts; ToolTipText: $"OptionsForm.frx":0264 | L=120, T=1720, W=3135, H=375 | Visible=0, TabIndex=194 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / RepopCooldownText` | `VB.TextBox` | Text: 1; ToolTipText: The minimum number of cycles that must elapse between repopulation events. | L=120, T=1380, W=480, H=285 | TabIndex=107 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / RepopAmountText` | `VB.TextBox` | Text: 10; ToolTipText: How many robots to drop in the sim per repopulation events. | L=120, T=1040, W=480, H=285 | TabIndex=106 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / MinVegText` | `VB.TextBox` | Text: 10; ToolTipText: $"OptionsForm.frx":032E | L=120, T=660, W=495, H=285 | TabIndex=105 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / MaxPopText` | `VB.TextBox` | Text: 1000 | L=120, T=280, W=480, H=285 | TabIndex=104 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / RepopAmountUpDown` | `ComCtl2.UpDown` | — | L=600, T=1040, W=255, H=285 | Index=0, Enabled=True, TabIndex=108 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / MaxPopUpDn` | `ComCtl2.UpDown` | — | L=600, T=280, W=255, H=285 | Enabled=True, Value=1, TabIndex=109 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / MinVegUpDn` | `ComCtl2.UpDown` | — | L=600, T=660, W=255, H=285 | Enabled=True, Value=100, TabIndex=110 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / RepopCooldownUpDown` | `ComCtl2.UpDown` | — | L=600, T=1380, W=255, H=285 | Index=1, Enabled=True, TabIndex=111 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / LabelZZ` | `VB.Label` | Caption: * 16K Maximum Chloroplasts; ToolTipText: An average Vegy has 16000 Chloroplasts. | L=960, T=315, W=2295, H=195 | Index=0, TabIndex=115 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / LabelZZ` | `VB.Label` | Caption: * 16K Repopulation threshold of Chloroplasts; ToolTipText: An average Vegy has 16000 Chloroplasts. | L=960, T=600, W=2055, H=435 | Index=1, TabIndex=114 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / LabelZZ` | `VB.Label` | Caption: Robots per repopulation event | L=960, T=1100, W=2235, H=195 | Index=2, TabIndex=113 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame29 / LabelZZ` | `VB.Label` | Caption: Repopulation cooldown period | L=960, T=1440, W=2235, H=195 | Index=3, TabIndex=112 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame30` | `VB.Frame` | Caption: Initial light energy | L=3720, T=240, W=1695, H=735 | TabIndex=100 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame30 / MaxNRGText` | `VB.TextBox` | Text: 40; ToolTipText: Amount of energy to give each veggy each cycle.  Is overrided by Pond Mode settings if that's on. | L=180, T=300, W=735, H=285 | TabIndex=101 |
| `SSTab1 / GenPropFrame / VeggyControls / Frame30 / VegNRG` | `VB.Label` | Caption: NRG | L=960, T=360, W=690, H=195 | TabIndex=102 |
| `SSTab1 / GenPropFrame / Frame18` | `VB.Frame` | Caption: Corpse Mode | L=2280, T=1200, W=1815, H=2955 | TabIndex=86 |
| `SSTab1 / GenPropFrame / Frame18 / CorpseCheck` | `VB.CheckBox` | Caption: Enable; ToolTipText: Enable corpse mode.  Corpses are dead robots who still have carcasses that can be eaten by other robots. | L=360, T=240, W=1185, H=255 | TabIndex=98, Alignment=1 |
| `SSTab1 / GenPropFrame / Frame18 / Frame3` | `VB.Frame` | Caption: Decay Rate | L=240, T=1920, W=1455, H=1005 | TabIndex=91 |
| `SSTab1 / GenPropFrame / Frame18 / Frame3 / FrequencyText` | `VB.TextBox` | Text: 1; ToolTipText: How many cycles per shot? | L=600, T=600, W=435, H=285 | TabIndex=93 |
| `SSTab1 / GenPropFrame / Frame18 / Frame3 / DecayText` | `VB.TextBox` | Text: 2; ToolTipText: How large is the decay shot? | L=600, T=240, W=435, H=285 | TabIndex=92 |
| `SSTab1 / GenPropFrame / Frame18 / Frame3 / DecayUpDn` | `ComCtl2.UpDown` | ToolTipText: Set how fast you want your corpses to decay away. | L=1080, T=240, W=255, H=285 | Index=0, Enabled=True, TabIndex=94 |
| `SSTab1 / GenPropFrame / Frame18 / Frame3 / DecayUpDn` | `ComCtl2.UpDown` | ToolTipText: Set how fast you want your corpses to decay away. | L=1080, T=600, W=255, H=285 | Index=1, Enabled=True, TabIndex=95 |
| `SSTab1 / GenPropFrame / Frame18 / Frame3 / FrequencyLabel1` | `VB.Label` | Caption: Period | L=90, T=660, W=810, H=255 | TabIndex=97 |
| `SSTab1 / GenPropFrame / Frame18 / Frame3 / Label59` | `VB.Label` | Caption: Size | L=90, T=300, W=630, H=255 | TabIndex=96 |
| `SSTab1 / GenPropFrame / Frame18 / Frame9` | `VB.Frame` | Caption: Decay Type | L=240, T=600, W=1455, H=1215 | TabIndex=87 |
| `SSTab1 / GenPropFrame / Frame18 / Frame9 / DecayOption` | `VB.OptionButton` | Caption: None | L=120, T=240, W=1095, H=255 | Index=0, TabIndex=90 |
| `SSTab1 / GenPropFrame / Frame18 / Frame9 / DecayOption` | `VB.OptionButton` | Caption: Waste | L=120, T=540, W=1095, H=255 | Index=1, TabIndex=89 |
| `SSTab1 / GenPropFrame / Frame18 / Frame9 / DecayOption` | `VB.OptionButton` | Caption: NRG | L=120, T=840, W=1095, H=255 | Index=2, TabIndex=88 |
| `SSTab1 / GenPropFrame / Frame2` | `VB.Frame` | Caption: Graphing | L=2280, T=4200, W=1815, H=975 | TabIndex=81 |
| `SSTab1 / GenPropFrame / Frame2 / ChartInterval` | `VB.TextBox` | Text: 200; ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=720, T=480, W=615, H=285 | TabIndex=82 |
| `SSTab1 / GenPropFrame / Frame2 / ChartingUpDown5` | `ComCtl2.UpDown` | ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=4320, T=3240, W=255, H=255 | Enabled=True, Value=200, TabIndex=83 |
| `SSTab1 / GenPropFrame / Frame2 / UpDown4` | `ComCtl2.UpDown` | ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=1440, T=480, W=255, H=285 | Enabled=True, Value=100, TabIndex=84 |
| `SSTab1 / GenPropFrame / Frame2 / Label17` | `VB.Label` | Caption: Update Interval | L=120, T=360, W=855, H=375 | TabIndex=85 |
| `SSTab1 / GenPropFrame / Label27` | `VB.Label` | Caption: Energy per veggie per cycle | L=1260, T=360, W=2055, H=210 | Visible=0, TabIndex=149 |
| `SSTab1 / DuplicaButt` | `VB.CommandButton` | Caption: Duplicate; ToolTipText: Add a new robot type to the simulation | L=1440, T=3720, W=1095, H=375 | TabIndex=78 |
| `SSTab1 / SpecList` | `VB.ListBox` | — | L=240, T=720, W=3465, H=2400 | TabIndex=77 |
| `SSTab1 / AddSpec` | `VB.CommandButton` | Caption: Add; ToolTipText: Add a new robot type to the simulation | L=240, T=3240, W=1095, H=375 | TabIndex=76 |
| `SSTab1 / DelSpec` | `VB.CommandButton` | Caption: Delete | L=240, T=3720, W=1095, H=375 | TabIndex=75 |
| `SSTab1 / RenameButton` | `VB.CommandButton` | Caption: Rename | L=2640, T=3720, W=1095, H=375 | TabIndex=74 |
| `SSTab1 / Frame14` | `VB.Frame` | Caption: Oscillation Frequency | L=-69720, T=1320, W=4335, H=2115 | TabIndex=39 |
| `SSTab1 / Frame14 / MutOscillSin` | `VB.CheckBox` | Caption: Sine wave instead of saw wave | L=360, T=1080, W=1095, H=615 | TabIndex=211 |
| `SSTab1 / Frame14 / CyclesHi` | `VB.TextBox` | Text: 10000 | L=2850, T=1005, W=765, H=330 | TabIndex=42 |
| `SSTab1 / Frame14 / CyclesLo` | `VB.TextBox` | Text: 100000 | L=2850, T=1500, W=795, H=330 | TabIndex=41 |
| `SSTab1 / Frame14 / MutOscill` | `VB.CheckBox` | Caption: Rendi la frequenza di mutazione oscilante fra 20x e 1/20x, alternativamente. | L=240, T=360, W=3660, H=525 | TabIndex=40, Alignment=1 |
| `SSTab1 / Frame14 / CycLoUpDn` | `ComCtl2.UpDown` | — | L=3720, T=1005, W=255, H=330 | Enabled=True, Value=100000, TabIndex=43 |
| `SSTab1 / Frame14 / CycHiUpDn` | `ComCtl2.UpDown` | — | L=3720, T=1500, W=255, H=330 | Enabled=True, Value=10000, TabIndex=44 |
| `SSTab1 / Frame14 / Label3` | `VB.Label` | Caption: Cicli a 16x | L=1440, T=1065, W=1245, H=225 | TabIndex=46, Alignment=1 |
| `SSTab1 / Frame14 / Label9` | `VB.Label` | Caption: Cycles at 1/16x | L=1335, T=1560, W=1320, H=195 | TabIndex=45, Alignment=1 |
| `SSTab1 / Frame13` | `VB.Frame` | Caption: Current Multiplier | L=-74700, T=1320, W=4755, H=2115 | TabIndex=32 |
| `SSTab1 / Frame13 / MutSlide` | `MSComctlLib.Slider` | — | L=360, T=420, W=4215, H=225 | TabIndex=33 |
| `SSTab1 / Frame13 / Label31` | `VB.Label` | Caption: $"OptionsForm.frx":03C7 | L=300, T=1020, W=4350, H=975 | BorderStyle=1, TabIndex=38 |
| `SSTab1 / Frame13 / Label30` | `VB.Label` | Caption: 1 | L=2400, T=720, W=255, H=255 | TabIndex=37 |
| `SSTab1 / Frame13 / MutLab` | `VB.Label` | Caption: 1 X | L=2280, T=150, W=975, H=255 | TabIndex=36 |
| `SSTab1 / Frame13 / Label15` | `VB.Label` | Caption: alta | L=4305, T=720, W=375, H=255 | TabIndex=35 |
| `SSTab1 / Frame13 / Label14` | `VB.Label` | Caption: bassa | L=225, T=720, W=495, H=255 | TabIndex=34 |
| `SSTab1 / Frame7` | `VB.Frame` | Caption: Contest Conditions | L=-74760, T=600, W=4935, H=2355 | TabIndex=21 |
| `SSTab1 / Frame7 / MaxPopF1Text` | `VB.TextBox` | — | L=3000, T=1980, W=735, H=285 | TabIndex=198 |
| `SSTab1 / Frame7 / F1Check` | `VB.CheckBox` | Caption: Use Contest Mode; ToolTipText: Sets screen size, costs and conditions for F1 match | L=330, T=360, W=2865, H=255 | TabIndex=25, Alignment=1 |
| `SSTab1 / Frame7 / txtMinRounds` | `VB.TextBox` | Text: 5 | L=3000, T=720, W=480, H=285 | TabIndex=24 |
| `SSTab1 / Frame7 / txtMaxRounds` | `VB.TextBox` | Text: 0; ToolTipText: Write zero here to turn option off | L=3000, T=1140, W=465, H=285 | TabIndex=23 |
| `SSTab1 / Frame7 / MaxCyclesText` | `VB.TextBox` | — | L=3000, T=1560, W=1215, H=285 | TabIndex=22 |
| `SSTab1 / Frame7 / FrequencyCheckUpDn` | `ComCtl2.UpDown` | — | L=3480, T=1140, W=255, H=285 | Enabled=True, Value=5, TabIndex=26 |
| `SSTab1 / Frame7 / ContestsUpDn` | `ComCtl2.UpDown` | — | L=3480, T=720, W=255, H=285 | Enabled=True, Value=5, TabIndex=27 |
| `SSTab1 / Frame7 / ContestNumText` | `VB.Label` | Caption: Number of Rounds in a Contest            XXX         rounds; ToolTipText: How many rounds of combat? | L=360, T=780, W=3975, H=255 | TabIndex=31 |
| `SSTab1 / Frame7 / MaxRoundsToDrawLabel` | `VB.Label` | Caption: Determine winner if a robot won            XXX         rounds; ToolTipText: How many cycles between sampling | L=360, T=1200, W=4335, H=255 | TabIndex=30 |
| `SSTab1 / Frame7 / Label29` | `VB.Label` | Caption: Max cycles in a round           at least     XXXXXXXXX   cycles | L=360, T=1620, W=4410, H=195 | TabIndex=29 |
| `SSTab1 / Frame7 / Label49` | `VB.Label` | Caption: Max population per contestent            XXXXXX    robots | L=360, T=2025, W=4290, H=255 | TabIndex=28 |
| `SSTab1 / Restart` | `VB.Frame` | Caption: General Restart Conditions | L=-69660, T=600, W=4455, H=2355 | TabIndex=19 |
| `SSTab1 / Restart / RestartSimCheck` | `VB.CheckBox` | Caption: Restart Sim when all Robots are Dead; ToolTipText: Just what it says | L=240, T=360, W=3135, H=255 | TabIndex=20, Alignment=1 |
| `SSTab1 / Simulazione` | `VB.Frame` | Caption: Internet Mode Settings | L=-74760, T=480, W=3885, H=4995 | TabIndex=18 |
| `SSTab1 / Simulazione / txtInttPort` | `VB.TextBox` | — | L=240, T=3960, W=3015, H=285 | TabIndex=207 |
| `SSTab1 / Simulazione / txtInttServ` | `VB.TextBox` | — | L=240, T=3240, W=3015, H=285 | TabIndex=204 |
| `SSTab1 / Simulazione / IntName` | `VB.TextBox` | — | L=240, T=2400, W=3015, H=285 | TabIndex=192 |
| `SSTab1 / Simulazione / outboundPathText` | `VB.TextBox` | — | L=240, T=1560, W=3135, H=285 | TabIndex=190 |
| `SSTab1 / Simulazione / inboundPathText` | `VB.TextBox` | — | L=240, T=720, W=3135, H=285 | TabIndex=187 |
| `SSTab1 / Simulazione / Label46` | `VB.Label` | Caption: Server Port | L=240, T=3600, W=1095, H=255 | TabIndex=208 |
| `SSTab1 / Simulazione / Label45` | `VB.Label` | Caption: Server IP/URL | L=240, T=2880, W=1095, H=255 | TabIndex=205 |
| `SSTab1 / Simulazione / Label40` | `VB.Label` | Caption: User Name | L=240, T=2040, W=1095, H=255 | TabIndex=191 |
| `SSTab1 / Simulazione / Label39` | `VB.Label` | Caption: Outbound Path | L=240, T=1200, W=1215, H=255 | TabIndex=189 |
| `SSTab1 / Simulazione / Label1` | `VB.Label` | Caption: Inbound Path | L=240, T=360, W=1455, H=255 | TabIndex=188 |
| `SSTab1 / Frame21` | `VB.Frame` | Caption: Costs and Returned Shots | L=-70080, T=420, W=4815, H=5235 | TabIndex=6 |
| `SSTab1 / Frame21 / PropUpDn` | `ComCtl2.UpDown` | — | L=3960, T=3000, W=255, H=285 | Visible=0, Enabled=True, TabIndex=14 |
| `SSTab1 / Frame21 / ExchangeProp` | `VB.OptionButton` | Caption: Proportional | L=2160, T=3000, W=1185, H=225 | TabIndex=12 |
| `SSTab1 / Frame21 / ExchangeFix` | `VB.OptionButton` | Caption: Fixed Nrg | L=2160, T=2520, W=1125, H=225 | TabIndex=11 |
| `SSTab1 / Frame21 / Fixed` | `VB.TextBox` | Text: 0 | L=3480, T=2520, W=510, H=285 | TabIndex=10 |
| `SSTab1 / Frame21 / Prop` | `VB.TextBox` | Text: 0 | L=3480, T=3000, W=510, H=285 | TabIndex=9 |
| `SSTab1 / Frame21 / FixUpDn` | `ComCtl2.UpDown` | — | L=3960, T=2520, W=255, H=285 | Enabled=True, TabIndex=13 |
| `SSTab1 / Frame21 / Frame28` | `VB.Frame` | Caption: Costs | L=120, T=300, W=4575, H=1935 | TabIndex=7 |
| `SSTab1 / Frame21 / Frame28 / CostRadio` | `VB.OptionButton` | Caption: Custom | L=600, T=1200, W=1215, H=495 | Index=2, TabIndex=171 |
| `SSTab1 / Frame21 / Frame28 / CostRadio` | `VB.OptionButton` | Caption: F1 Default | L=600, T=720, W=1215, H=495 | Index=1, TabIndex=170 |
| `SSTab1 / Frame21 / Frame28 / CostRadio` | `VB.OptionButton` | Caption: No Costs | L=600, T=240, W=1215, H=495 | Index=0, TabIndex=169 |
| `SSTab1 / Frame21 / Frame28 / ToCosts` | `VB.CommandButton` | Caption: Custom Simulation Costs | L=2040, T=1320, W=1875, H=375 | TabIndex=8 |
| `SSTab1 / Frame21 / Label8` | `VB.Label` | Caption: $"OptionsForm.frx":0482 | L=225, T=3840, W=4335, H=945 | BorderStyle=1, TabIndex=17 |
| `SSTab1 / Frame21 / Label16` | `VB.Label` | Caption: Shot Energy Exchange Method | L=225, T=2640, W=1905, H=495 | TabIndex=16 |
| `SSTab1 / Frame21 / Label10` | `VB.Label` | Caption: % | L=4320, T=3050, W=135, H=195 | TabIndex=15 |
| `SSTab1 / CommentBox` | `RichTextLib.RichTextBox` | — | L=240, T=4680, W=3495, H=975 | TabIndex=79, ScrollBars=2 |
| `SSTab1 / Frame1` | `VB.Frame` | Caption: Species Properties | L=3840, T=600, W=6075, H=5085 | TabIndex=47 |
| `SSTab1 / Frame1 / btnFatalRES` | `VB.CommandButton` | Caption: Fatal Restrictions | L=3260, T=4480, W=1400, H=360 | TabIndex=206 |
| `SSTab1 / Frame1 / chkNoChlr` | `VB.CheckBox` | Caption: Disable Chloroplasts; ToolTipText: Feed automatically this type (only for vegetables) | L=3360, T=2400, W=2430, H=210 | TabIndex=203, Alignment=1 |
| `SSTab1 / Frame1 / MutEnabledCheck` | `VB.CheckBox` | Caption: Disable Mutations; ToolTipText: Disables voluntary movement for this species | L=3360, T=4080, W=2430, H=330 | TabIndex=184, Alignment=1 |
| `SSTab1 / Frame1 / VirusImmuneCheck` | `VB.CheckBox` | Caption: Virus Immune; ToolTipText: Disables voluntary movement for this species | L=3360, T=3840, W=2430, H=330 | TabIndex=183, Alignment=1 |
| `SSTab1 / Frame1 / DisableReproductionCheck` | `VB.CheckBox` | Caption: Disable Reproduction; ToolTipText: Disables voluntary movement for this species | L=3360, T=3600, W=2430, H=330 | TabIndex=182, Alignment=1 |
| `SSTab1 / Frame1 / DisableMovementSysvarsCheck` | `VB.CheckBox` | Caption: Disable Voluntary Movement; ToolTipText: Disables voluntary movement for this species | L=3360, T=3360, W=2430, H=330 | TabIndex=181, Alignment=1 |
| `SSTab1 / Frame1 / DisableDNACheck` | `VB.CheckBox` | Caption: Disable DNA Execution; ToolTipText: Speeds up the simulation by turning off DNA execution for this species | L=3360, T=3120, W=2430, H=330 | TabIndex=180, Alignment=1 |
| `SSTab1 / Frame1 / DisableVisionCheck` | `VB.CheckBox` | Caption: Disable Vision; ToolTipText: Speeds up the simulation by turning off vision for this species | L=3360, T=2880, W=2430, H=330 | TabIndex=179, Alignment=1 |
| `SSTab1 / Frame1 / InNrg` | `VB.CommandButton` | Caption: 30000 | L=5280, T=1800, W=615, H=255 | Index=3, TabIndex=67 |
| `SSTab1 / Frame1 / InNrg` | `VB.CommandButton` | Caption: 5000 | L=4680, T=1800, W=615, H=255 | Index=2, TabIndex=66 |
| `SSTab1 / Frame1 / InNrg` | `VB.CommandButton` | Caption: 3000 | L=4080, T=1800, W=615, H=255 | Index=0, TabIndex=65 |
| `SSTab1 / Frame1 / IndNum` | `VB.CommandButton` | Caption: 30 | L=5400, T=840, W=495, H=255 | Index=3, TabIndex=64 |
| `SSTab1 / Frame1 / IndNum` | `VB.CommandButton` | Caption: 15 | L=4920, T=840, W=495, H=255 | Index=2, TabIndex=63 |
| `SSTab1 / Frame1 / IndNum` | `VB.CommandButton` | Caption: 5 | L=4440, T=840, W=495, H=255 | Index=0, TabIndex=62 |
| `SSTab1 / Frame1 / BlockSpec` | `VB.CheckBox` | Caption: Fixed in place | L=3360, T=2640, W=2430, H=330 | TabIndex=61, Alignment=1 |
| `SSTab1 / Frame1 / SpecVeg` | `VB.CheckBox` | Caption: Repopulate; ToolTipText: Feed automatically this type (only for vegetables) | L=3360, T=2160, W=2430, H=210 | TabIndex=58, Alignment=1 |
| `SSTab1 / Frame1 / SpecQty` | `VB.TextBox` | Text: 0 | L=5040, T=480, W=585, H=285 | TabIndex=57, Alignment=1 |
| `SSTab1 / Frame1 / SpecCol` | `VB.ComboBox` | ToolTipText: Initial colour | L=1350, T=1770, W=1380, H=315 | Style=2, TabIndex=56 |
| `SSTab1 / Frame1 / SpecNrg` | `VB.TextBox` | Text: 0 | L=5040, T=1440, W=585, H=285 | TabIndex=55, Alignment=1 |
| `SSTab1 / Frame1 / MutRatesBut` | `VB.CommandButton` | Caption: Mutation Rates; ToolTipText: Sets the initial mutation rates for this type | L=4700, T=4480, W=1215, H=360 | TabIndex=54 |
| `SSTab1 / Frame1 / IndNum` | `VB.CommandButton` | Caption: 0 | L=3960, T=840, W=495, H=255 | Index=4, TabIndex=53 |
| `SSTab1 / Frame1 / PosReset` | `VB.CommandButton` | Caption: Reset | L=1680, T=2280, W=1095, H=375 | TabIndex=52 |
| `SSTab1 / Frame1 / UpDown1` | `ComCtl2.UpDown` | ToolTipText: Set the initial number of copies for this robot type | L=5640, T=480, W=240, H=285 | Enabled=True, TabIndex=68 |
| `SSTab1 / Frame1 / UpDown3` | `ComCtl2.UpDown` | ToolTipText: Initial energy assigned to this type | L=5640, T=1440, W=240, H=285 | Enabled=True, Value=1000, TabIndex=69 |
| `SSTab1 / Frame1 / Frame6` | `VB.Frame` | Caption: Skin | L=195, T=390, W=2565, H=1200 | TabIndex=59 |
| `SSTab1 / Frame1 / Frame6 / SkinChange` | `VB.CommandButton` | Caption: Change | L=285, T=450, W=855, H=330 | TabIndex=60 |
| `SSTab1 / Frame1 / Frame6 / Cerchio` | `VB.Shape` | — | L=1515, T=315, W=645, H=645 | — |
| `SSTab1 / Frame1 / Frame6 / Shape3` | `VB.Shape` | — | L=2115, T=555, W=150, H=165 | — |
| `SSTab1 / Frame1 / Frame6 / Line9` | `VB.Line` | — | — | — |
| `SSTab1 / Frame1 / Frame6 / Line8` | `VB.Line` | — | — | — |
| `SSTab1 / Frame1 / Frame6 / Line7` | `VB.Line` | — | — | — |
| `SSTab1 / Frame1 / Frame6 / Shape2` | `VB.Shape` | — | L=1410, T=210, W=900, H=885 | — |
| `SSTab1 / Frame1 / IPB` | `VB.PictureBox` | — | L=120, T=2760, W=2775, H=1695 | TabIndex=48 |
| `SSTab1 / Frame1 / IPB / Initial_Position` | `VB.PictureBox` | — | L=120, T=120, W=2535, H=1455 | TabIndex=51 |
| `SSTab1 / Frame1 / IPB / RobPlacLine` | `VB.PictureBox` | — | L=600, T=960, W=1215, H=135 | Index=0, Visible=0, BorderStyle=0, TabIndex=50, Appearance=0 |
| `SSTab1 / Frame1 / IPB / picHandle` | `VB.PictureBox` | — | L=1200, T=960, W=255, H=255 | Index=0, Visible=0, BorderStyle=0, TabIndex=49, Appearance=0 |
| `SSTab1 / Frame1 / Line4` | `VB.Line` | — | — | — |
| `SSTab1 / Frame1 / Label2` | `VB.Label` | Caption: Individui | L=3360, T=480, W=855, H=255 | TabIndex=73 |
| `SSTab1 / Frame1 / Label4` | `VB.Label` | Caption: Posizione iniziale | L=165, T=2400, W=1575, H=255 | TabIndex=72 |
| `SSTab1 / Frame1 / Label5` | `VB.Label` | Caption: Colore | L=360, T=1830, W=750, H=225 | TabIndex=71 |
| `SSTab1 / Frame1 / Label6` | `VB.Label` | Caption: Energia iniziale | L=3360, T=1440, W=1455, H=255 | TabIndex=70 |
| `SSTab1 / Label19` | `VB.Label` | Caption: $"OptionsForm.frx":0635 | L=-74640, T=3600, W=6135, H=495 | TabIndex=201 |
| `SSTab1 / Label42` | `VB.Label` | Caption: $"OptionsForm.frx":06CF | L=-70560, T=1560, W=5655, H=975 | TabIndex=195 |
| `SSTab1 / Label41` | `VB.Label` | Caption: $"OptionsForm.frx":0804 | L=-70560, T=600, W=5775, H=855 | TabIndex=193 |
| `SSTab1 / Label36` | `VB.Label` | Caption: Commenti sulla specie: | L=240, T=4440, W=2055, H=255 | TabIndex=164 |
| `SSTab1 / SpeciesLabel` | `VB.Label` | Caption: Native Species: | L=240, T=480, W=3375, H=255 | TabIndex=163 |
| `Label13` | `VB.Label` | Caption: Label13 | L=4440, T=3000, W=1215, H=495 | TabIndex=165 |

## Network event (`NetEvent`)

Source: [`NetEvent.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/NetEvent.frm)  
Class: `VB.Form`. Client size: `2205 × 1095` twips. Border style: `0`. Startup position: `default`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Timer1` | `VB.Timer` | — | L=1755, T=630 | — |
| `Label1` | `VB.Label` | Caption: close | L=780, T=840, W=555, H=195 | TabIndex=2, Alignment=2 |
| `Netlab2` | `VB.Label` | Caption: Something happened! | L=90, T=105, W=2010, H=500 | TabIndex=1, Alignment=2 |
| `Shape1` | `VB.Shape` | — | L=0, T=0, W=2215, H=1095 | — |
| `NetLab` | `VB.Label` | Caption: Something happened! | L=105, T=135, W=2010, H=500 | TabIndex=0, Alignment=2 |

## Form2 (`grafico`)

Source: [`grafico.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/grafico.frm)  
Class: `VB.Form`. Client size: `7995 × 10950` twips. Border style: `default`. Startup position: `default`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `chk_GDsave` | `VB.CheckBox` | Caption: Save on d.p. 999; ToolTipText: Saves graph data to file(s) on every data point 999 | L=6600, T=5520, W=1695, H=255 | TabIndex=139 |
| `ResetButton` | `VB.CommandButton` | Caption: Reset; ToolTipText: Updates graph now without waiting for next update interval | L=5400, T=5520, W=1095, H=255 | TabIndex=138, TabStop=0 |
| `UpdateNow` | `VB.CommandButton` | Caption: Update Now; ToolTipText: Updates graph now without waiting for next update interval | L=4200, T=5520, W=1095, H=255 | TabIndex=12, TabStop=0 |
| `secret_exit` | `VB.Label` | Caption: T | L=0, T=0, W=375, H=375 | TabIndex=140 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=5760, W=540, H=255 | Index=65, TabIndex=137 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=5880, W=540, H=255 | Index=64, TabIndex=136 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=6000, W=540, H=255 | Index=63, TabIndex=135 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=6120, W=540, H=255 | Index=62, TabIndex=134 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=6240, W=540, H=255 | Index=61, TabIndex=133 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=6360, W=540, H=255 | Index=60, TabIndex=132 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=6480, W=540, H=255 | Index=59, TabIndex=131 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=6600, W=540, H=255 | Index=58, TabIndex=130 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=6840, W=540, H=255 | Index=57, TabIndex=129 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=6960, W=540, H=255 | Index=56, TabIndex=128 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=7080, W=540, H=255 | Index=55, TabIndex=127 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=7200, W=540, H=255 | Index=54, TabIndex=126 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=7320, W=540, H=255 | Index=53, TabIndex=125 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=7440, W=540, H=255 | Index=52, TabIndex=124 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=7560, W=540, H=255 | Index=51, TabIndex=123 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=7680, W=540, H=255 | Index=50, TabIndex=122 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=7800, W=540, H=255 | Index=49, TabIndex=121 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=7920, W=540, H=255 | Index=48, TabIndex=120 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=8040, W=540, H=255 | Index=47, TabIndex=119 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=8160, W=540, H=255 | Index=46, TabIndex=118 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=8280, W=540, H=255 | Index=45, TabIndex=117 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=11160, W=540, H=255 | Index=44, Visible=0, TabIndex=116 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=11040, W=540, H=255 | Index=43, Visible=0, TabIndex=115 |
| `popnum` | `VB.Label` | Caption: 10 | L=2400, T=10920, W=540, H=255 | Index=42, Visible=0, TabIndex=114 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=5520, W=3495, H=255 | Index=67, Visible=0, TabIndex=113 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=5760, W=3495, H=255 | Index=66, Visible=0, TabIndex=112 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=6000, W=3495, H=255 | Index=65, Visible=0, TabIndex=111 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=6240, W=3495, H=255 | Index=64, Visible=0, TabIndex=110 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=6480, W=3495, H=255 | Index=63, Visible=0, TabIndex=109 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=6720, W=3495, H=255 | Index=62, Visible=0, TabIndex=108 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=6960, W=3495, H=255 | Index=61, Visible=0, TabIndex=107 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=7200, W=3495, H=255 | Index=60, Visible=0, TabIndex=106 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=7440, W=3495, H=255 | Index=59, Visible=0, TabIndex=105 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=7680, W=3495, H=255 | Index=58, Visible=0, TabIndex=104 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=7920, W=3495, H=255 | Index=57, Visible=0, TabIndex=103 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=8235, W=3495, H=255 | Index=56, Visible=0, TabIndex=102 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=8475, W=3495, H=255 | Index=55, Visible=0, TabIndex=101 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=8715, W=3495, H=255 | Index=54, Visible=0, TabIndex=100 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=8955, W=3495, H=255 | Index=53, Visible=0, TabIndex=99 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=9195, W=3495, H=255 | Index=52, Visible=0, TabIndex=98 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=9435, W=3495, H=255 | Index=51, Visible=0, TabIndex=97 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=9675, W=3495, H=255 | Index=50, Visible=0, TabIndex=96 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=9915, W=3495, H=255 | Index=49, Visible=0, TabIndex=95 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=10155, W=3495, H=255 | Index=48, Visible=0, TabIndex=94 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=10275, W=3495, H=255 | Index=47, Visible=0, TabIndex=93 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=11640, W=3495, H=255 | Index=46, Visible=0, TabIndex=92 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=11400, W=3495, H=255 | Index=45, Visible=0, TabIndex=91 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=11160, W=3495, H=255 | Index=44, Visible=0, TabIndex=90 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1080, T=10875, W=3495, H=255 | Index=43, Visible=0, TabIndex=89 |
| `Label1` | `VB.Label` | Caption: Label1 | L=1125, T=10680, W=3495, H=255 | Index=42, Visible=0, TabIndex=88 |
| `Shape3` | `VB.Shape` | — | L=645, T=5595, W=135, H=135 | Index=66, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=5835, W=135, H=135 | Index=65, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=6075, W=135, H=135 | Index=64, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=6315, W=135, H=135 | Index=63, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=6555, W=135, H=135 | Index=62, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=6795, W=135, H=135 | Index=61, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=7035, W=135, H=135 | Index=60, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=7275, W=135, H=135 | Index=59, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=7515, W=135, H=135 | Index=58, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=7755, W=135, H=135 | Index=57, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=7995, W=135, H=135 | Index=56, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=8190, W=135, H=135 | Index=55, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=8430, W=135, H=135 | Index=54, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=8670, W=135, H=135 | Index=53, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=8910, W=135, H=135 | Index=52, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=9150, W=135, H=135 | Index=51, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=9390, W=135, H=135 | Index=50, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=9750, W=135, H=135 | Index=49, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=9990, W=135, H=135 | Index=48, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=10350, W=135, H=135 | Index=47, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=600, T=10590, W=135, H=135 | Index=46, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=11475, W=135, H=135 | Index=45, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=11235, W=135, H=135 | Index=44, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=10995, W=135, H=135 | Index=43, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=645, T=10755, W=135, H=135 | Index=42, Visible=0 |
| `YLab` | `VB.Label` | Caption: 0 | L=1920, T=9480, W=1095, H=255 | Index=1, TabIndex=87 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=5205, W=3495, H=255 | Index=41, Visible=0, TabIndex=86 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=5400, W=3495, H=255 | Index=40, Visible=0, TabIndex=85 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=5685, W=3495, H=255 | Index=39, Visible=0, TabIndex=84 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=5925, W=3495, H=255 | Index=38, Visible=0, TabIndex=83 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=6165, W=3495, H=255 | Index=37, Visible=0, TabIndex=82 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=6405, W=3495, H=255 | Index=36, Visible=0, TabIndex=81 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=6645, W=3495, H=255 | Index=35, Visible=0, TabIndex=80 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=6885, W=3495, H=255 | Index=34, Visible=0, TabIndex=79 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=7125, W=3495, H=255 | Index=33, Visible=0, TabIndex=78 |
| `Shape3` | `VB.Shape` | — | L=6165, T=5205, W=135, H=135 | Index=41, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=5445, W=135, H=135 | Index=40, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=5685, W=135, H=135 | Index=39, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=5925, W=135, H=135 | Index=38, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=6165, W=135, H=135 | Index=37, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=6405, W=135, H=135 | Index=36, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=6645, W=135, H=135 | Index=35, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=6885, W=135, H=135 | Index=34, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=7125, W=135, H=135 | Index=33, Visible=0 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=7365, W=3495, H=255 | Index=32, Visible=0, TabIndex=77 |
| `Shape3` | `VB.Shape` | — | L=6165, T=7365, W=135, H=135 | Index=32, Visible=0 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=7605, W=3495, H=255 | Index=31, Visible=0, TabIndex=76 |
| `Shape3` | `VB.Shape` | — | L=6165, T=7605, W=135, H=135 | Index=31, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=7800, W=135, H=135 | Index=30, Visible=0 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=7920, W=3495, H=255 | Index=30, Visible=0, TabIndex=75 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=8160, W=3495, H=255 | Index=29, Visible=0, TabIndex=74 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=8400, W=3495, H=255 | Index=28, Visible=0, TabIndex=73 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=8640, W=3495, H=255 | Index=27, Visible=0, TabIndex=72 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=8880, W=3495, H=255 | Index=26, Visible=0, TabIndex=71 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=9120, W=3495, H=255 | Index=25, Visible=0, TabIndex=70 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=9360, W=3495, H=255 | Index=24, Visible=0, TabIndex=69 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=9600, W=3495, H=255 | Index=23, Visible=0, TabIndex=68 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=9840, W=3495, H=255 | Index=22, Visible=0, TabIndex=67 |
| `Shape3` | `VB.Shape` | — | L=6120, T=8040, W=135, H=135 | Index=29, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=8280, W=135, H=135 | Index=28, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=8520, W=135, H=135 | Index=27, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=8760, W=135, H=135 | Index=26, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=9000, W=135, H=135 | Index=25, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=9360, W=135, H=135 | Index=24, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=9600, W=135, H=135 | Index=23, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=9960, W=135, H=135 | Index=22, Visible=0 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=5160, W=540, H=255 | Index=41, Visible=0, TabIndex=66 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=5280, W=540, H=255 | Index=40, Visible=0, TabIndex=65 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=5400, W=540, H=255 | Index=39, Visible=0, TabIndex=64 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=5520, W=540, H=255 | Index=38, Visible=0, TabIndex=63 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=5640, W=540, H=255 | Index=37, Visible=0, TabIndex=62 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=5760, W=540, H=255 | Index=36, Visible=0, TabIndex=61 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=5880, W=540, H=255 | Index=35, Visible=0, TabIndex=60 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=6000, W=540, H=255 | Index=34, Visible=0, TabIndex=59 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=6240, W=540, H=255 | Index=33, Visible=0, TabIndex=58 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=6360, W=540, H=255 | Index=32, Visible=0, TabIndex=57 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=6480, W=540, H=255 | Index=31, Visible=0, TabIndex=56 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=6600, W=540, H=255 | Index=30, Visible=0, TabIndex=55 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=6720, W=540, H=255 | Index=29, Visible=0, TabIndex=54 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=6840, W=540, H=255 | Index=28, Visible=0, TabIndex=53 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=6960, W=540, H=255 | Index=27, Visible=0, TabIndex=52 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=7080, W=540, H=255 | Index=26, Visible=0, TabIndex=51 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=7200, W=540, H=255 | Index=25, Visible=0, TabIndex=50 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=7320, W=540, H=255 | Index=24, Visible=0, TabIndex=49 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=7440, W=540, H=255 | Index=23, Visible=0, TabIndex=48 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=7560, W=540, H=255 | Index=22, Visible=0, TabIndex=47 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=9960, W=3495, H=255 | Index=21, Visible=0, TabIndex=46 |
| `Shape3` | `VB.Shape` | — | L=6120, T=10200, W=135, H=135 | Index=21, Visible=0 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=7680, W=540, H=255 | Index=21, Visible=0, TabIndex=45 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=2520, W=540, H=255 | Index=20, TabIndex=44 |
| `Shape3` | `VB.Shape` | — | L=6120, T=5040, W=135, H=135 | Index=20, Visible=0 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=4800, W=3495, H=255 | Index=20, Visible=0, TabIndex=43 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=2400, W=540, H=255 | Index=19, TabIndex=42 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=2280, W=540, H=255 | Index=18, TabIndex=41 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=2160, W=540, H=255 | Index=17, TabIndex=40 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=2040, W=540, H=255 | Index=16, TabIndex=39 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=1920, W=540, H=255 | Index=15, TabIndex=38 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=1800, W=540, H=255 | Index=14, TabIndex=37 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=1680, W=540, H=255 | Index=13, TabIndex=36 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=1560, W=540, H=255 | Index=12, TabIndex=35 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=1440, W=540, H=255 | Index=11, TabIndex=34 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=1320, W=540, H=255 | Index=10, TabIndex=33 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=1200, W=540, H=255 | Index=9, TabIndex=32 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=1080, W=540, H=255 | Index=8, TabIndex=31 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=840, W=540, H=255 | Index=7, TabIndex=30 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=720, W=540, H=255 | Index=6, TabIndex=29 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=600, W=540, H=255 | Index=5, TabIndex=28 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=480, W=540, H=255 | Index=4, TabIndex=27 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=360, W=540, H=255 | Index=3, TabIndex=26 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=240, W=540, H=255 | Index=2, TabIndex=25 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=120, W=540, H=255 | Index=1, TabIndex=24 |
| `popnum` | `VB.Label` | Caption: 10 | L=5520, T=0, W=540, H=255 | Index=0, TabIndex=23 |
| `Shape3` | `VB.Shape` | — | L=6120, T=4800, W=135, H=135 | Index=19, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=4440, W=135, H=135 | Index=18, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=4200, W=135, H=135 | Index=17, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=3840, W=135, H=135 | Index=16, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=3600, W=135, H=135 | Index=15, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=3360, W=135, H=135 | Index=14, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=3120, W=135, H=135 | Index=13, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6120, T=2880, W=135, H=135 | Index=12, Visible=0 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=4680, W=3500, H=255 | Index=19, Visible=0, TabIndex=22 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=4440, W=3500, H=255 | Index=18, Visible=0, TabIndex=21 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=4200, W=3500, H=255 | Index=17, Visible=0, TabIndex=20 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=3960, W=3500, H=255 | Index=16, Visible=0, TabIndex=19 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=3720, W=3500, H=255 | Index=15, Visible=0, TabIndex=18 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=3480, W=3500, H=255 | Index=14, Visible=0, TabIndex=17 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=3240, W=3500, H=255 | Index=13, Visible=0, TabIndex=16 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=3000, W=3500, H=255 | Index=12, Visible=0, TabIndex=15 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6360, T=2760, W=3500, H=255 | Index=11, Visible=0, TabIndex=14 |
| `Shape3` | `VB.Shape` | — | L=6120, T=2640, W=135, H=135 | Index=11, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=2445, W=135, H=135 | Index=10, Visible=0 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=2445, W=3500, H=255 | Index=10, Visible=0, TabIndex=13 |
| `YLab` | `VB.Label` | Caption: 0 | L=1920, T=10440, W=1095, H=255 | Index=0, TabIndex=11 |
| `XLabel` | `VB.Label` | Caption: Cycles | L=120, T=5520, W=4695, H=195 | TabIndex=10 |
| `Shape3` | `VB.Shape` | — | L=6165, T=2205, W=135, H=135 | Index=9, Visible=0 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=2205, W=3500, H=255 | Index=9, Visible=0, TabIndex=9 |
| `Shape3` | `VB.Shape` | — | L=6165, T=1965, W=135, H=135 | Index=8, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=1725, W=135, H=135 | Index=7, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=1485, W=135, H=135 | Index=6, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=1245, W=135, H=135 | Index=5, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=1005, W=135, H=135 | Index=4, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=765, W=135, H=135 | Index=3, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=525, W=135, H=135 | Index=2, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=285, W=135, H=135 | Index=1, Visible=0 |
| `Shape3` | `VB.Shape` | — | L=6165, T=45, W=135, H=135 | Index=0, Visible=0 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=1965, W=3500, H=255 | Index=8, Visible=0, TabIndex=8 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=1725, W=3500, H=255 | Index=7, Visible=0, TabIndex=7 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=1485, W=3500, H=255 | Index=6, Visible=0, TabIndex=6 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=1245, W=3500, H=255 | Index=5, Visible=0, TabIndex=5 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=1005, W=3500, H=255 | Index=4, Visible=0, TabIndex=4 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=765, W=3500, H=255 | Index=3, Visible=0, TabIndex=3 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=525, W=3500, H=255 | Index=2, Visible=0, TabIndex=2 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=285, W=3500, H=255 | Index=1, Visible=0, TabIndex=1 |
| `Label1` | `VB.Label` | Caption: Label1 | L=6405, T=45, W=3500, H=255 | Index=0, Visible=0, TabIndex=0 |
| `Riquadro` | `VB.Shape` | — | L=120, T=30, W=5355, H=5355 | Visible=0 |

## Genes Activations (`ActivForm`)

Source: [`ActivForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/ActivForm.frm)  
Class: `VB.Form`. Client size: `5715 × 990` twips. Border style: `4`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `pbox` | `VB.PictureBox` | — | L=0, T=810, W=5715, H=240 | BorderStyle=0, TabIndex=0, Appearance=0 |
| `cornice2` | `VB.Shape` | — | L=0, T=0, W=5715, H=570 | — |
| `cornice` | `VB.Shape` | — | L=0, T=555, W=5715, H=240 | — |

## Form1 (`Form1`)

Source: [`main.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/main.frm)  
Class: `VB.Form`. Client size: `12045 × 8445` twips. Border style: `0`. Startup position: `default`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `SecTimer` | `VB.Timer` | — | L=2040, T=120 | — |
| `CommonDialog1` | `MSComDlg.CommonDialog` | — | L=120, T=120 | — |
| `PlayerBot` | `VB.Label` | Caption: PlayerBot Mode | L=120, T=120, W=1935, H=255 | Visible=0, TabIndex=6 |
| `lblSaving` | `VB.Label` | — | L=4440, T=4680, W=4215, H=615 | Visible=0, TabIndex=5 |
| `lblSafeMode` | `VB.Label` | Caption: Safe Mode | L=600, T=4680, W=3255, H=615 | Visible=0, TabIndex=4 |
| `GraphLab` | `VB.Label` | Caption: Updating Graph: 0% | L=600, T=4200, W=4935, H=255 | Visible=0, TabIndex=3 |
| `BoyLabl` | `VB.Label` | Caption: $"main.frx":08CA | L=600, T=3480, W=9855, H=615 | Visible=0, TabIndex=2 |
| `InternetModePopFileProblem` | `VB.Image` | — | L=5280, T=5400, W=240, H=255 | Visible=0 |
| `InternetModeBackPressure` | `VB.Image` | — | L=5280, T=5040, W=240, H=255 | Visible=0 |
| `InternetModeStart` | `VB.Image` | — | L=5280, T=4680, W=240, H=255 | Visible=0 |
| `InternetModeOff` | `VB.Image` | — | L=5280, T=4320, W=240, H=255 | Visible=0 |
| `ServerGood` | `VB.Image` | — | L=5280, T=3600, W=240, H=255 | Visible=0 |
| `ServerBad` | `VB.Image` | — | L=5280, T=3960, W=240, H=255 | Visible=0 |
| `InternetMode` | `VB.Label` | Caption: Internet Mode | L=600, T=2760, W=3225, H=495 | Visible=0, TabIndex=1 |
| `TeleporterMask` | `VB.Image` | — | L=240, T=7560, W=11520, H=720 | Visible=0 |
| `Teleporter` | `VB.Image` | — | L=240, T=6600, W=11520, H=720 | Visible=0, Appearance=0 |
| `Label1` | `VB.Label` | Caption: Output video interrotto... premi sul pulsante nella barra in alto per ripristinarlo. | L=600, T=1440, W=10665, H=1335 | Visible=0, TabIndex=0 |

## Contest Results (`Contest_Form`)

Source: [`Contest_Form.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Contest_Form.frm)  
Class: `VB.Form`. Client size: `4935 × 2295` twips. Border style: `default`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Option1` | `VB.OptionButton` | Caption: W | L=4260, T=1680, W=315, H=255 | Index=5, Style=1, TabIndex=30 |
| `Option1` | `VB.OptionButton` | Caption: W | L=4260, T=1440, W=315, H=255 | Index=4, Style=1, TabIndex=29 |
| `Option1` | `VB.OptionButton` | Caption: W | L=4260, T=1200, W=315, H=255 | Index=3, Style=1, TabIndex=28 |
| `Option1` | `VB.OptionButton` | Caption: W | L=4260, T=960, W=315, H=255 | Index=2, Style=1, TabIndex=27 |
| `Option1` | `VB.OptionButton` | Caption: W | L=4260, T=720, W=315, H=255 | Index=1, Style=1, TabIndex=26 |
| `Option1` | `VB.OptionButton` | — | L=5160, T=1980, W=315, H=255 | Index=0, Style=1, TabIndex=25 |
| `Label4` | `VB.Label` | Caption: The 0th option1 box is hidden behind screen --->>>>> | L=780, T=1980, W=3795, H=255 | Visible=0, TabIndex=31 |
| `Label3` | `VB.Label` | Caption: InstaWin; ToolTipText: Give this round to this robot. | L=4140, T=240, W=675, H=195 | TabIndex=24 |
| `Maxrounds` | `VB.Label` | Caption: 5 | L=2520, T=0, W=735, H=255 | TabIndex=23 |
| `Label2` | `VB.Label` | Caption: Round | L=720, T=0, W=495, H=255 | TabIndex=22 |
| `Winner1` | `VB.Label` | — | L=120, T=480, W=975, H=255 | TabIndex=21 |
| `Winner` | `VB.Label` | — | L=1200, T=480, W=3375, H=255 | TabIndex=20 |
| `Pop5` | `VB.Label` | Caption: 0 | L=3600, T=1680, W=615, H=255 | TabIndex=19 |
| `Pop4` | `VB.Label` | Caption: 0 | L=3600, T=1440, W=615, H=255 | TabIndex=18 |
| `Pop3` | `VB.Label` | Caption: 0 | L=3600, T=1200, W=615, H=255 | TabIndex=17 |
| `Pop2` | `VB.Label` | Caption: 0 | L=3600, T=960, W=615, H=255 | TabIndex=16 |
| `Pop1` | `VB.Label` | Caption: 0 | L=3600, T=720, W=615, H=255 | TabIndex=15 |
| `PopLabel` | `VB.Label` | Caption: Population | L=3240, T=240, W=855, H=255 | TabIndex=14 |
| `Wins5` | `VB.Label` | Caption: Wins5 | L=2760, T=1680, W=735, H=255 | TabIndex=13 |
| `Wins4` | `VB.Label` | Caption: Wins4 | L=2760, T=1440, W=735, H=255 | TabIndex=12 |
| `Robname5` | `VB.Label` | Caption: Robname5 | L=120, T=1680, W=2535, H=255 | TabIndex=11 |
| `Robname4` | `VB.Label` | Caption: Robname4 | L=120, T=1440, W=2535, H=255 | TabIndex=10 |
| `Wins3` | `VB.Label` | Caption: Wins3 | L=2760, T=1200, W=735, H=255 | TabIndex=9 |
| `Wins2` | `VB.Label` | Caption: Wins2 | L=2760, T=960, W=735, H=255 | TabIndex=8 |
| `Robname3` | `VB.Label` | Caption: Robname3 | L=120, T=1200, W=2535, H=255 | TabIndex=7 |
| `Robname2` | `VB.Label` | Caption: Robname2 | L=120, T=960, W=2535, H=255 | TabIndex=6 |
| `Contests` | `VB.Label` | Caption: 1 | L=1320, T=0, W=495, H=255 | TabIndex=5, Alignment=2 |
| `Label1` | `VB.Label` | Caption: of | L=2040, T=0, W=255, H=255 | TabIndex=4 |
| `wins1` | `VB.Label` | Caption: Wins1 | L=2760, T=720, W=735, H=255 | TabIndex=3 |
| `Robname1` | `VB.Label` | Caption: Robname1 | L=120, T=720, W=2535, H=255 | TabIndex=2 |
| `Wins` | `VB.Label` | Caption: Wins | L=2640, T=240, W=615, H=255 | TabIndex=1 |
| `RoName` | `VB.Label` | Caption: Robot Name | L=600, T=240, W=1335, H=255 | TabIndex=0 |

## Help for DNA commands (`DNA_Help`)

Source: [`frmAbout1.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmAbout1.frm)  
Class: `VB.Form`. Client size: `10455 × 7335` twips. Border style: `3`. Startup position: `default`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Basic` | `VB.CommandButton` | Caption: &Basic Help | L=6600, T=6840, W=1740, H=345 | TabIndex=4 |
| `Help` | `VB.TextBox` | — | L=240, T=840, W=9975, H=5835 | TabIndex=3, ScrollBars=2 |
| `picIcon` | `VB.PictureBox` | — | L=240, T=240, W=540, H=540 | TabIndex=1 |
| `cmdOK` | `VB.CommandButton` | Caption: &Exit Help | L=8520, T=6840, W=1740, H=345 | TabIndex=0 |
| `lblTitle` | `VB.Label` | Caption: Application Title | L=1050, T=240, W=3885, H=480 | TabIndex=2 |

## Mutation Probabilities (`MutationsProbability`)

Source: [`NeoMutprob.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/NeoMutprob.frm)  
Class: `VB.Form`. Client size: `9165 × 8490` twips. Border style: `default`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Command1` | `VB.CommandButton` | Caption: Default Rates | L=3600, T=7920, W=1455, H=495 | Index=0, TabIndex=65 |
| `Probs` | `VB.TextBox` | Text: Text1 | L=1320, T=7020, W=855, H=285 | Index=2, Visible=0, Enabled=0, TabIndex=60 |
| `Probs` | `VB.TextBox` | Text: Text1 | L=1320, T=6600, W=855, H=285 | Index=1, Enabled=0, TabIndex=59 |
| `Text2` | `VB.TextBox` | Text: "NeoMutprob.frx":08CA | L=3420, T=120, W=5595, H=855 | TabIndex=57, ScrollBars=2 |
| `CustGauss` | `VB.Frame` | Caption: Custom Gauss | L=6300, T=1740, W=2715, H=2775 | Index=1, TabIndex=39 |
| `CustGauss / Picture1` | `VB.PictureBox` | — | L=180, T=420, W=2295, H=1155 | Index=1, TabIndex=44 |
| `CustGauss / Upper` | `VB.TextBox` | Text: 32000 | L=1740, T=1920, W=555, H=285 | Index=1, TabIndex=43 |
| `CustGauss / Lower` | `VB.TextBox` | Text: 32000 | L=300, T=1920, W=555, H=285 | Index=1, TabIndex=42 |
| `CustGauss / Mean` | `VB.TextBox` | Text: 32000 | L=1020, T=1920, W=555, H=285 | Index=1, TabIndex=41 |
| `CustGauss / StdDev` | `VB.TextBox` | Text: 5.268 | L=1740, T=2280, W=555, H=285 | Index=1, TabIndex=40 |
| `CustGauss / Label2` | `VB.Label` | Caption: Ints | L=2340, T=1980, W=315, H=195 | Index=7, Visible=0, TabIndex=49 |
| `CustGauss / Label1` | `VB.Label` | Caption: Upper | L=1800, T=1620, W=495, H=255 | Index=6, TabIndex=48 |
| `CustGauss / Label1` | `VB.Label` | Caption: Lower | L=360, T=1620, W=435, H=255 | Index=5, TabIndex=47 |
| `CustGauss / Label1` | `VB.Label` | Caption: Mean | L=1080, T=1620, W=435, H=255 | Index=4, TabIndex=46 |
| `CustGauss / Label4` | `VB.Label` | Caption: Standard Deviation:  | L=300, T=2340, W=1515, H=255 | Index=1, TabIndex=45 |
| `Command1` | `VB.CommandButton` | Caption: Okay | L=1860, T=7920, W=1455, H=495 | Index=1, TabIndex=36 |
| `MutTypeEnabled` | `VB.CheckBox` | Caption: Enabled | L=3420, T=1140, W=2055, H=435 | Style=1, TabIndex=35 |
| `Probs` | `VB.TextBox` | Text: 1234567 | L=6630, T=1180, W=755, H=285 | Index=0, TabIndex=33, Alignment=1 |
| `Global` | `VB.CheckBox` | Caption: Apply Changes Globally | L=180, T=7560, W=1995, H=195 | Visible=0, TabIndex=32 |
| `CustGauss` | `VB.Frame` | Caption: Custom Gauss | L=3420, T=1740, W=2715, H=2775 | Index=0, TabIndex=23 |
| `CustGauss / StdDev` | `VB.TextBox` | Text: 5.268 | L=1740, T=2280, W=555, H=285 | Index=0, TabIndex=38 |
| `CustGauss / Mean` | `VB.TextBox` | Text: 32000 | L=1020, T=1920, W=555, H=285 | Index=0, TabIndex=28 |
| `CustGauss / Lower` | `VB.TextBox` | Text: 32000 | L=300, T=1920, W=555, H=285 | Index=0, TabIndex=27 |
| `CustGauss / Upper` | `VB.TextBox` | Text: 32000 | L=1740, T=1920, W=555, H=285 | Index=0, TabIndex=26 |
| `CustGauss / Picture1` | `VB.PictureBox` | — | L=180, T=420, W=2295, H=1155 | Index=0, TabIndex=24 |
| `CustGauss / Label4` | `VB.Label` | Caption: Standard Deviation:  | L=300, T=2340, W=1515, H=255 | Index=0, TabIndex=37 |
| `CustGauss / Label1` | `VB.Label` | Caption: Mean | L=1080, T=1620, W=435, H=255 | Index=3, TabIndex=31 |
| `CustGauss / Label1` | `VB.Label` | Caption: Lower | L=360, T=1620, W=435, H=255 | Index=1, TabIndex=30 |
| `CustGauss / Label1` | `VB.Label` | Caption: Upper | L=1800, T=1620, W=495, H=255 | Index=2, TabIndex=29 |
| `CustGauss / Label2` | `VB.Label` | Caption: Ints | L=2340, T=1980, W=315, H=195 | Index=6, Visible=0, TabIndex=25 |
| `Slider1` | `MSComctlLib.Slider` | — | L=5160, T=5040, W=2235, H=495 | TabIndex=8 |
| `Frame2` | `VB.Frame` | Caption: Mutation Types | L=120, T=120, W=3135, H=5055 | TabIndex=7 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Copy Error 2 | L=240, T=4320, W=1215, H=495 | Index=10, Style=1, TabIndex=67 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Point 2 | L=1680, T=3540, W=1215, H=495 | Index=9, Style=1, TabIndex=66 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Translocation | L=240, T=3540, W=1215, H=495 | Index=8, Style=1, TabIndex=64 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Delta Mutation | L=1680, T=420, W=1215, H=495 | Index=7, Style=1, TabIndex=58 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Point | L=240, T=420, W=1215, H=495 | Index=0, Style=1, TabIndex=56 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Minor Deletion | L=1680, T=1980, W=1215, H=495 | Index=1, Style=1, TabIndex=55 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Reversal | L=240, T=1980, W=1215, H=495 | Index=2, Style=1, TabIndex=54 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Insertion | L=1680, T=1200, W=1215, H=495 | Index=3, Style=1, TabIndex=53 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Amplification | L=240, T=2760, W=1215, H=495 | Index=4, Style=1, TabIndex=52 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Major Deletion | L=1680, T=2760, W=1215, H=495 | Index=5, Style=1, TabIndex=51 |
| `Frame2 / TypeOption` | `VB.OptionButton` | Caption: Copy Error | L=240, T=1200, W=1215, H=495 | Index=6, Style=1, TabIndex=50 |
| `Sliders6` | `VB.Frame` | — | L=7920, T=5640, W=5415, H=2715 | Visible=0, BorderStyle=0, TabIndex=0 |
| `Sliders6 / TypeProb` | `MSComctlLib.Slider` | — | L=240, T=300, W=630, H=2115 | Index=0, TabIndex=1 |
| `Sliders6 / TypeProb` | `MSComctlLib.Slider` | — | L=1110, T=300, W=630, H=2115 | Index=1, TabIndex=2 |
| `Sliders6 / TypeProb` | `MSComctlLib.Slider` | — | L=1965, T=300, W=630, H=2115 | Index=2, TabIndex=3 |
| `Sliders6 / TypeProb` | `MSComctlLib.Slider` | — | L=2835, T=300, W=630, H=2115 | Index=3, TabIndex=4 |
| `Sliders6 / TypeProb` | `MSComctlLib.Slider` | — | L=3690, T=300, W=630, H=2115 | Index=4, TabIndex=5 |
| `Sliders6 / TypeProb` | `MSComctlLib.Slider` | — | L=4560, T=300, W=630, H=2115 | Index=5, TabIndex=6 |
| `Sliders6 / TypePerc` | `VB.Label` | Caption: 100% | L=3816, T=2400, W=435, H=195 | Index=5, TabIndex=20 |
| `Sliders6 / TypePerc` | `VB.Label` | Caption: 100% | L=4680, T=2400, W=435, H=195 | Index=4, TabIndex=19 |
| `Sliders6 / TypePerc` | `VB.Label` | Caption: 100% | L=2952, T=2400, W=435, H=195 | Index=3, TabIndex=18 |
| `Sliders6 / TypePerc` | `VB.Label` | Caption: 100% | L=2088, T=2400, W=435, H=195 | Index=2, TabIndex=17 |
| `Sliders6 / TypePerc` | `VB.Label` | Caption: 100% | L=1224, T=2400, W=435, H=195 | Index=1, TabIndex=16 |
| `Sliders6 / TypePerc` | `VB.Label` | Caption: 100% | L=360, T=2400, W=435, H=195 | Index=0, TabIndex=15 |
| `Sliders6 / Label2` | `VB.Label` | Caption: Flow | L=4680, T=60, W=435, H=195 | Index=5, TabIndex=14 |
| `Sliders6 / Label2` | `VB.Label` | Caption: Comparison | L=3600, T=60, W=855, H=195 | Index=4, TabIndex=13 |
| `Sliders6 / Label2` | `VB.Label` | Caption: Logic | L=2940, T=60, W=435, H=195 | Index=3, TabIndex=12 |
| `Sliders6 / Label2` | `VB.Label` | Caption: Operator | L=1920, T=60, W=735, H=195 | Index=2, TabIndex=11 |
| `Sliders6 / Label2` | `VB.Label` | Caption: *Number | L=1080, T=60, W=675, H=195 | Index=1, TabIndex=10 |
| `Sliders6 / Label2` | `VB.Label` | Caption: Number | L=240, T=60, W=615, H=195 | Index=0, TabIndex=9 |
| `Label5` | `VB.Label` | Caption: Mutation Sumations: | L=660, T=6240, W=1995, H=255 | TabIndex=63 |
| `Label3` | `VB.Label` | Caption: 1 chance in  XXXXXXX.  per bot | L=420, T=7050, W=2415, H=255 | Index=2, Visible=0, TabIndex=62 |
| `Label3` | `VB.Label` | Caption: 1 chance in  XXXXXXX.  per bp | L=420, T=6630, W=2415, H=255 | Index=1, TabIndex=61 |
| `Label3` | `VB.Label` | Caption: 1 chance in  XXXXXXX per XXXXXXXX | L=5700, T=1215, W=3435, H=255 | Index=0, TabIndex=34 |
| `Slider1Text` | `VB.Label` | Caption: Change value | L=7560, T=5040, W=1215, H=495 | Index=1, TabIndex=22 |
| `Slider1Text` | `VB.Label` | Caption: Change type | L=3720, T=5040, W=1275, H=495 | Index=0, TabIndex=21 |

## Advanced Physics Options (`PhysicsOptions`)

Source: [`PhysicsOptions.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/PhysicsOptions.frm)  
Class: `VB.Form`. Client size: `9330 × 5850` twips. Border style: `1`. Startup position: `1`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Command` | `VB.CommandButton` | Caption: Okay | L=7320, T=5280, W=1815, H=435 | Index=2, TabIndex=24 |
| `Frame4` | `VB.Frame` | Caption: Misc | L=4980, T=120, W=4215, H=3375 | TabIndex=11 |
| `Frame4 / Frame6` | `VB.Frame` | Caption: Planet Eaters | L=2340, T=2220, W=1755, H=1095 | TabIndex=25 |
| `Frame4 / Frame6 / Toggles` | `VB.CheckBox` | Caption: Planet Eaters | L=180, T=300, W=1275, H=315 | Index=1, TabIndex=27 |
| `Frame4 / Frame6 / PlanetEatersText` | `VB.TextBox` | Text: 100 | L=420, T=630, W=555, H=285 | TabIndex=26 |
| `Frame4 / Frame6 / Label4` | `VB.Label` | Caption: G:  XXXXX E+3 | L=180, T=660, W=1215, H=255 | TabIndex=28 |
| `Frame4 / Frame5` | `VB.Frame` | Caption: Toggles | L=120, T=2220, W=2175, H=1095 | TabIndex=21 |
| `Frame4 / Frame5 / Toggles` | `VB.CheckBox` | Caption: Zero Momentum | L=180, T=480, W=1515, H=315 | Index=0, TabIndex=22 |
| `Frame4 / MiscSlider` | `MSComctlLib.Slider` | — | L=1380, T=240, W=2355, H=615 | Index=0, TabIndex=13 |
| `Frame4 / MiscSlider` | `MSComctlLib.Slider` | — | L=1380, T=900, W=2355, H=615 | Index=1, TabIndex=16 |
| `Frame4 / MiscSlider` | `MSComctlLib.Slider` | — | L=1380, T=1620, W=2355, H=615 | Index=2, TabIndex=19 |
| `Frame4 / MiscText` | `VB.Label` | Caption: 0 | L=3720, T=1800, W=435, H=315 | Index=2, TabIndex=20 |
| `Frame4 / Label3` | `VB.Label` | Caption: Brownian Motion | L=180, T=1800, W=1215, H=255 | Index=5, TabIndex=18 |
| `Frame4 / MiscText` | `VB.Label` | Caption: 0 | L=3720, T=1080, W=435, H=315 | Index=1, TabIndex=17 |
| `Frame4 / Label3` | `VB.Label` | Caption: Y axis Gravity | L=180, T=1080, W=1155, H=255 | Index=4, TabIndex=15 |
| `Frame4 / MiscText` | `VB.Label` | Caption: 0% | L=3720, T=420, W=435, H=315 | Index=0, TabIndex=14 |
| `Frame4 / Label3` | `VB.Label` | Caption: Bang Efficiency | L=180, T=420, W=1155, H=255 | Index=3, TabIndex=12 |
| `Command` | `VB.CommandButton` | Caption: General Help | L=3720, T=5280, W=1815, H=435 | Index=3, TabIndex=10 |
| `Frame3` | `VB.Frame` | Caption: Friction | L=120, T=2700, W=4695, H=2415 | TabIndex=9 |
| `Frame3 / Friction` | `MSComctlLib.Slider` | — | L=1440, T=240, W=2595, H=615 | Index=0, TabIndex=29 |
| `Frame3 / Friction` | `MSComctlLib.Slider` | — | L=1440, T=990, W=2595, H=615 | Index=2, TabIndex=30 |
| `Frame3 / Friction` | `MSComctlLib.Slider` | — | L=1440, T=1740, W=2595, H=615 | Index=1, TabIndex=35 |
| `Frame3 / Label3` | `VB.Label` | Caption: Kinetic Coefficient | L=120, T=1920, W=1395, H=255 | Index=1, TabIndex=37 |
| `Frame3 / FrictionText` | `VB.Label` | Caption: 0 | L=4080, T=1920, W=435, H=195 | Index=1, TabIndex=36 |
| `Frame3 / Label3` | `VB.Label` | Caption: Z axis Gravity | L=120, T=420, W=1275, H=255 | Index=0, TabIndex=34 |
| `Frame3 / Label3` | `VB.Label` | Caption: Static Coefficient | L=120, T=1170, W=1275, H=255 | Index=2, TabIndex=33 |
| `Frame3 / FrictionText` | `VB.Label` | Caption: 0 | L=4080, T=360, W=435, H=195 | Index=0, TabIndex=32 |
| `Frame3 / FrictionText` | `VB.Label` | Caption: 0 | L=4080, T=1170, W=435, H=195 | Index=2, TabIndex=31 |
| `Command` | `VB.CommandButton` | Caption: Help on Units | L=120, T=5280, W=1815, H=435 | Index=0, TabIndex=8 |
| `Text1` | `VB.TextBox` | Text: "PhysicsOptions.frx":08CA | L=4980, T=3660, W=4215, H=1455 | TabIndex=6, ScrollBars=2 |
| `Frame1` | `VB.Frame` | Caption: Fluid Dynamics | L=120, T=120, W=4695, H=2415 | TabIndex=0 |
| `Frame1 / Command1` | `VB.CommandButton` | Caption: ? | L=2760, T=1680, W=255, H=255 | TabIndex=23 |
| `Frame1 / FluidText` | `VB.TextBox` | Text: 10000 | L=1560, T=1680, W=1215, H=255 | Index=2, Enabled=0, TabIndex=7 |
| `Frame1 / FluidText` | `VB.TextBox` | Text: 10 | L=1020, T=840, W=975, H=315 | Index=1, TabIndex=4 |
| `Frame1 / FluidText` | `VB.TextBox` | Text: 100 | L=1020, T=420, W=975, H=315 | Index=0, TabIndex=3 |
| `Frame1 / Label2` | `VB.Label` | Caption: Reynold's Number for an average bot moving at 1 twip/cycle: | L=240, T=1440, W=3195, H=435 | Index=0, TabIndex=5 |
| `Frame1 / Label1` | `VB.Label` | Caption: Density:    XXXXXXXXXE-7 Mass per cubic twip | L=300, T=900, W=4155, H=255 | Index=1, TabIndex=2 |
| `Frame1 / Label1` | `VB.Label` | Caption: Viscosity:  XXXXXXXXXE-5 Bangs per square twips | L=300, T=480, W=4095, H=255 | Index=0, TabIndex=1 |

## Costs (`CostsForm`)

Source: [`CostsForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/CostsForm.frm)  
Class: `VB.Form`. Client size: `10920 × 9255` twips. Border style: `1`. Startup position: `1`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Frame2` | `VB.Frame` | Caption: Cost Overrides | L=120, T=4920, W=5655, H=4095 | TabIndex=70 |
| `Frame2 / AllowNegativeCostXCheck` | `VB.CheckBox` | Caption: Allow Multiplier to go Negative | L=3000, T=360, W=2535, H=255 | TabIndex=101 |
| `Frame2 / DynamicCostsIncludePlantsCheck` | `VB.CheckBox` | Caption: Include all robots | L=240, T=1800, W=1695, H=255 | TabIndex=100 |
| `Frame2 / CostX` | `VB.TextBox` | Text: 1.0; ToolTipText: This value is applied to the costs to determine the actual costs per cycle | L=1560, T=360, W=1185, H=285 | TabIndex=98 |
| `Frame2 / CostReinstate` | `VB.TextBox` | Text: 0; ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=4320, T=1440, W=960, H=285 | TabIndex=92 |
| `Frame2 / Frame3` | `VB.Frame` | Caption: Dynamic Cost Adjustment | L=120, T=2160, W=5415, H=1815 | TabIndex=74 |
| `Frame2 / Frame3 / DynamicCostsRangeL` | `VB.TextBox` | Text: 100; ToolTipText: Target bot population ignores corpses, walls, veggies | L=4440, T=1080, W=375, H=285 | TabIndex=87 |
| `Frame2 / Frame3 / DynamicCostsRangeU` | `VB.TextBox` | Text: 100; ToolTipText: Target bot population ignores corpses, walls, veggies | L=4440, T=720, W=375, H=285 | TabIndex=83 |
| `Frame2 / Frame3 / DynamicCostTargetPopulation` | `VB.TextBox` | Text: 5000; ToolTipText: Target bot population ignores corpses, walls, veggies | L=4320, T=360, W=465, H=285 | TabIndex=80 |
| `Frame2 / Frame3 / DynamicCosts` | `VB.CheckBox` | Caption: Enable Dynamic Costs | L=240, T=240, W=1935, H=255 | TabIndex=79 |
| `Frame2 / Frame3 / DynamicCostSensitivitySlider` | `MSComctlLib.Slider` | — | L=120, T=840, W=2175, H=555 | Value=1, TabIndex=75 |
| `Frame2 / Frame3 / DynamicCostsUpDown` | `ComCtl2.UpDown` | — | L=4800, T=360, W=255, H=285 | Enabled=True, Value=100, TabIndex=81 |
| `Frame2 / Frame3 / UpDown1` | `ComCtl2.UpDown` | — | L=4800, T=720, W=255, H=285 | Enabled=True, Value=100, TabIndex=84 |
| `Frame2 / Frame3 / UpDown3` | `ComCtl2.UpDown` | — | L=4800, T=1080, W=255, H=285 | Enabled=True, Value=100, TabIndex=88 |
| `Frame2 / Frame3 / Label15` | `VB.Label` | Caption: %; ToolTipText: Target includes only extant hetertrophs | L=5160, T=1155, W=135, H=255 | TabIndex=90 |
| `Frame2 / Frame3 / Label14` | `VB.Label` | Caption: Lower Range: Target -; ToolTipText: Target includes only extant hetertrophs | L=2640, T=1080, W=1695, H=255 | TabIndex=89 |
| `Frame2 / Frame3 / Label13` | `VB.Label` | Caption: %; ToolTipText: Target includes only extant hetertrophs | L=5160, T=795, W=135, H=255 | TabIndex=86 |
| `Frame2 / Frame3 / Label12` | `VB.Label` | Caption: Upper Range: Target +; ToolTipText: Target includes only extant hetertrophs | L=2640, T=720, W=1695, H=255 | TabIndex=85 |
| `Frame2 / Frame3 / Label8` | `VB.Label` | Caption: Target Population; ToolTipText: Target includes only extant hetertrophs | L=2640, T=360, W=1455, H=255 | TabIndex=82 |
| `Frame2 / Frame3 / Label7` | `VB.Label` | Caption: Sensitivity | L=840, T=600, W=855, H=255 | TabIndex=78 |
| `Frame2 / Frame3 / Label10` | `VB.Label` | Caption: Low | L=240, T=1440, W=495, H=255 | TabIndex=77 |
| `Frame2 / Frame3 / Label11` | `VB.Label` | Caption: High | L=1800, T=1440, W=375, H=255 | TabIndex=76 |
| `Frame2 / BotNoCostThreshold` | `VB.TextBox` | Text: 0; ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=4320, T=1080, W=960, H=285 | TabIndex=71 |
| `Frame2 / UpDown2` | `ComCtl2.UpDown` | — | L=5280, T=1080, W=255, H=285 | Enabled=True, Value=100, TabIndex=72 |
| `Frame2 / UpDown4` | `ComCtl2.UpDown` | — | L=5280, T=1440, W=255, H=285 | Enabled=True, Value=100, TabIndex=93 |
| `Frame2 / Label19` | `VB.Label` | Caption: Cost Multiplier:; ToolTipText: Target includes only extant hetertrophs | L=240, T=375, W=1095, H=195 | TabIndex=99 |
| `Frame2 / Label16` | `VB.Label` | Caption: Reinstate Costs when population above | L=1320, T=1440, W=3015, H=255 | TabIndex=91 |
| `Frame2 / Label9` | `VB.Label` | Caption: Zero Costs when population falls below | L=1320, T=1080, W=2895, H=255 | TabIndex=73 |
| `AgeFrame` | `VB.Frame` | Caption: Aging | L=5880, T=6120, W=4095, H=2535 | TabIndex=62 |
| `AgeFrame / Costs` | `VB.TextBox` | Text: 0.0001; ToolTipText: Increase Age cost this amount per cycle once it begins | L=1920, T=1680, W=735, H=285 | Index=33, TabIndex=95 |
| `AgeFrame / LinearAgeCostCheck` | `VB.CheckBox` | Caption: Increase by | L=600, T=1680, W=1215, H=255 | TabIndex=94 |
| `AgeFrame / AgeCostLog` | `VB.CheckBox` | Caption: Increase log(bot age - cost start age) | L=600, T=2040, W=3255, H=375 | TabIndex=69 |
| `AgeFrame / Costs` | `VB.TextBox` | Text: Text1; ToolTipText: Don't begin charging the Age Cost until the bot reaches this age | L=1860, T=840, W=975, H=315 | Index=32, TabIndex=67 |
| `AgeFrame / Costs` | `VB.TextBox` | Text: Text1; ToolTipText: The cost per cycle in nrg which will be multiplied times log(age) and charged to the bot | L=1860, T=360, W=975, H=315 | Index=31, TabIndex=64 |
| `AgeFrame / Label18` | `VB.Label` | Caption: Once cost begins being applied: | L=120, T=1320, W=3405, H=255 | TabIndex=97 |
| `AgeFrame / Label17` | `VB.Label` | Caption: nrg per cycle | L=2760, T=1680, W=1215, H=255 | TabIndex=96 |
| `AgeFrame / Label6` | `VB.Label` | Caption: cycles old | L=2940, T=840, W=735, H=255 | TabIndex=68 |
| `AgeFrame / Label5` | `VB.Label` | Caption: Begins upon reaching | L=120, T=840, W=2535, H=255 | TabIndex=66 |
| `AgeFrame / Label4` | `VB.Label` | Caption: Age Cost | L=120, T=360, W=1725, H=255 | TabIndex=65 |
| `AgeFrame / Label3` | `VB.Label` | Caption: nrg per cycle | L=2940, T=360, W=975, H=255 | TabIndex=63 |
| `Default` | `VB.CommandButton` | Caption: F1 Default | L=7200, T=8760, W=1575, H=375 | TabIndex=61 |
| `ExitButton` | `VB.CommandButton` | Caption: Okay | L=9120, T=8760, W=1575, H=375 | TabIndex=54 |
| `Frame1` | `VB.Frame` | Caption: Morphological Costs | L=5880, T=120, W=4935, H=5955 | Index=2, TabIndex=25 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=5520, W=975, H=315 | Index=8, TabIndex=105 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=5040, W=975, H=315 | Index=25, TabIndex=102 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=4560, W=975, H=315 | Index=30, TabIndex=55 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=4080, W=975, H=315 | Index=29, TabIndex=34 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=3600, W=975, H=315 | Index=28, TabIndex=33 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=3120, W=975, H=315 | Index=27, TabIndex=32 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=2640, W=975, H=315 | Index=26, TabIndex=31 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=2160, W=975, H=315 | Index=24, TabIndex=30 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=1680, W=975, H=315 | Index=23, TabIndex=29 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=1200, W=975, H=315 | Index=22, TabIndex=28 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=720, W=975, H=315 | Index=21, TabIndex=27 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=2040, T=240, W=975, H=315 | Index=20, TabIndex=26 |
| `Frame1 / Label1` | `VB.Label` | Caption: Chloroplast Cost | L=300, T=5520, W=1275, H=255 | Index=10, TabIndex=107 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per added chlr. | L=3120, T=5520, W=1515, H=255 | Index=10, TabIndex=106 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per data per copy | L=3120, T=5040, W=1515, H=255 | Index=8, TabIndex=104 |
| `Frame1 / Label1` | `VB.Label` | Caption: DNA Copy | L=300, T=5040, W=1275, H=255 | Index=9, TabIndex=103 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per body per turn | L=3120, T=4560, W=1695, H=255 | Index=30, TabIndex=57 |
| `Frame1 / Label1` | `VB.Label` | Caption: Body Upkeep | L=300, T=4560, W=1095, H=255 | Index=30, TabIndex=56 |
| `Frame1 / Label1` | `VB.Label` | Caption: Voluntary Movement | L=300, T=240, W=1815, H=255 | Index=29, TabIndex=53 |
| `Frame1 / Label1` | `VB.Label` | Caption: Rotation | L=300, T=720, W=1635, H=255 | Index=28, TabIndex=52 |
| `Frame1 / Label1` | `VB.Label` | Caption: Shell Cost | L=300, T=4080, W=1095, H=255 | Index=27, TabIndex=51 |
| `Frame1 / Label1` | `VB.Label` | Caption: Tie Formation | L=300, T=1200, W=1335, H=255 | Index=26, TabIndex=50 |
| `Frame1 / Label1` | `VB.Label` | Caption: Shot Formation | L=300, T=1680, W=1875, H=255 | Index=25, TabIndex=49 |
| `Frame1 / Label1` | `VB.Label` | Caption: DNA Upkeep | L=300, T=2160, W=1275, H=255 | Index=24, TabIndex=48 |
| `Frame1 / Label1` | `VB.Label` | Caption: Venom Cost | L=300, T=2640, W=1095, H=255 | Index=22, TabIndex=47 |
| `Frame1 / Label1` | `VB.Label` | Caption: Poison Cost | L=300, T=3120, W=1095, H=255 | Index=21, TabIndex=46 |
| `Frame1 / Label1` | `VB.Label` | Caption: Slime Cost | L=300, T=3600, W=1095, H=255 | Index=20, TabIndex=45 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per bang | L=3120, T=240, W=1215, H=255 | Index=29, TabIndex=44 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per radian | L=3120, T=720, W=1215, H=255 | Index=28, TabIndex=43 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per firing | L=3120, T=1200, W=1215, H=255 | Index=27, TabIndex=42 |
| `Frame1 / Label2` | `VB.Label` | Caption: base nrg per shot | L=3120, T=1680, W=1335, H=255 | Index=26, TabIndex=41 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per data per cycle | L=3120, T=2160, W=1755, H=255 | Index=25, TabIndex=40 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per bp per copy | L=5280, T=840, W=1215, H=255 | Index=24, Visible=0, TabIndex=39 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per unit constructed | L=3120, T=2640, W=1695, H=255 | Index=23, TabIndex=38 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per unit constructed | L=3120, T=3120, W=1695, H=255 | Index=22, TabIndex=37 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per unit constructed | L=3120, T=3600, W=1695, H=255 | Index=21, TabIndex=36 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per unit constructed | L=3120, T=4080, W=1695, H=255 | Index=20, TabIndex=35 |
| `Frame1` | `VB.Frame` | Caption: DNA Command Costs | L=120, T=120, W=5655, H=4755 | Index=0, TabIndex=0 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=1965, T=1800, W=975, H=315 | Index=3, TabIndex=60 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=1965, T=360, W=975, H=315 | Index=0, TabIndex=59 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=1965, T=4260, W=975, H=315 | Index=9, TabIndex=58 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=1965, T=3765, W=975, H=315 | Index=7, TabIndex=6 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=1965, T=3270, W=975, H=315 | Index=6, TabIndex=5 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=1965, T=2790, W=975, H=315 | Index=5, TabIndex=4 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=1965, T=2280, W=975, H=315 | Index=4, TabIndex=3 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=1965, T=1335, W=975, H=315 | Index=2, TabIndex=2 |
| `Frame1 / Costs` | `VB.TextBox` | Text: Text1 | L=1965, T=840, W=975, H=315 | Index=1, TabIndex=1 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per instance | L=3120, T=4320, W=1215, H=255 | Index=9, TabIndex=24 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per instance | L=3120, T=3825, W=1215, H=255 | Index=7, TabIndex=23 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per instance | L=3120, T=3330, W=1215, H=255 | Index=6, TabIndex=22 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per instance | L=3120, T=2850, W=1215, H=255 | Index=5, TabIndex=21 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per instance | L=3120, T=2370, W=1215, H=255 | Index=4, TabIndex=20 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per instance | L=3120, T=1875, W=1215, H=255 | Index=3, TabIndex=19 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per instance | L=3120, T=1395, W=1215, H=255 | Index=2, TabIndex=18 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per instance | L=3120, T=900, W=1215, H=255 | Index=1, TabIndex=17 |
| `Frame1 / Label2` | `VB.Label` | Caption: nrg per instance | L=3120, T=420, W=1215, H=255 | Index=0, TabIndex=16 |
| `Frame1 / Label1` | `VB.Label` | Caption: Stores | L=180, T=3780, W=1095, H=255 | Index=8, TabIndex=15 |
| `Frame1 / Label1` | `VB.Label` | Caption: Logic | L=180, T=3300, W=1095, H=255 | Index=7, TabIndex=14 |
| `Frame1 / Label1` | `VB.Label` | Caption: Condition | L=180, T=2820, W=1095, H=255 | Index=6, TabIndex=13 |
| `Frame1 / Label1` | `VB.Label` | Caption: Bitwise Command | L=180, T=2340, W=1275, H=255 | Index=5, TabIndex=12 |
| `Frame1 / Label1` | `VB.Label` | Caption: Advanced Command | L=180, T=1860, W=1875, H=255 | Index=4, TabIndex=11 |
| `Frame1 / Label1` | `VB.Label` | Caption: Basic Command | L=180, T=1380, W=1335, H=255 | Index=3, TabIndex=10 |
| `Frame1 / Label1` | `VB.Label` | Caption: Flow Command | L=180, T=4320, W=1095, H=255 | Index=2, TabIndex=9 |
| `Frame1 / Label1` | `VB.Label` | Caption: *number | L=180, T=900, W=1095, H=255 | Index=1, TabIndex=8 |
| `Frame1 / Label1` | `VB.Label` | Caption: number | L=180, T=420, W=1095, H=255 | Index=0, TabIndex=7 |

## Energy Management (`EnergyForm`)

Source: [`EnergyForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/EnergyForm.frm)  
Class: `VB.Form`. Client size: `5610 × 5025` twips. Border style: `3`. Startup position: `default`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Frame1` | `VB.Frame` | Caption: Incoming Energy | L=120, T=120, W=5415, H=4335 | TabIndex=1 |
| `Frame1 / ffmTides` | `VB.Frame` | Caption: Tide Simulator (Advanced) | L=2040, T=720, W=3015, H=975 | TabIndex=18 |
| `Frame1 / ffmTides / txtTideOf` | `VB.TextBox` | Text: 0 | L=600, T=600, W=825, H=285 | TabIndex=22 |
| `Frame1 / ffmTides / txtTide` | `VB.TextBox` | Text: 0 | L=600, T=240, W=855, H=285 | TabIndex=19 |
| `Frame1 / ffmTides / TideUpDn` | `ComCtl2.UpDown` | ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=1560, T=240, W=255, H=285 | Enabled=True, Value=100, TabIndex=21 |
| `Frame1 / ffmTides / OfUpDown` | `ComCtl2.UpDown` | ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=1546, T=600, W=255, H=285 | Enabled=True, Value=100, TabIndex=23 |
| `Frame1 / ffmTides / Label3` | `VB.Label` | Caption: offset | L=120, T=645, W=495, H=255 | TabIndex=25 |
| `Frame1 / ffmTides / Label2` | `VB.Label` | Caption: cycles | L=1920, T=600, W=615, H=255 | TabIndex=24 |
| `Frame1 / ffmTides / lblTides` | `VB.Label` | Caption: cycles (off) | L=1920, T=240, W=855, H=255 | TabIndex=20 |
| `Frame1 / chkRnd` | `VB.CheckBox` | Caption: Enable Weather; ToolTipText: A cycle of light and darkness. Veggies are not fed or repopulated during night time. | L=240, T=840, W=2475, H=255 | TabIndex=17 |
| `Frame1 / Frame2` | `VB.Frame` | Caption: Thresholds | L=120, T=1680, W=5175, H=2535 | TabIndex=6 |
| `Frame1 / Frame2 / ThresholdMode` | `VB.OptionButton` | Caption: Advance Sun to Dawn / Dusk | L=2400, T=600, W=2535, H=255 | Index=2, TabIndex=16 |
| `Frame1 / Frame2 / ThresholdMode` | `VB.OptionButton` | Caption: Permanently Toggle Sun State | L=2400, T=960, W=2535, H=255 | Index=1, TabIndex=15 |
| `Frame1 / Frame2 / ThresholdMode` | `VB.OptionButton` | Caption: Temporarily Suspend Day Cycles | L=2400, T=240, W=2655, H=255 | Index=0, TabIndex=13 |
| `Frame1 / Frame2 / SunDown` | `VB.CheckBox` | Caption: Sun goes down if nrg > | L=480, T=1440, W=1935, H=255 | TabIndex=10 |
| `Frame1 / Frame2 / SunUp` | `VB.CheckBox` | Caption: Sun comes up if nrg < | L=480, T=1920, W=1935, H=255 | TabIndex=9 |
| `Frame1 / Frame2 / SunDownThreshold` | `VB.TextBox` | Text: 1000000; ToolTipText: Overrides day/night if configured and forces the sun down if the sim energy is above this value | L=2640, T=1440, W=1380, H=285 | TabIndex=8, Alignment=1 |
| `Frame1 / Frame2 / SunUpThreshold` | `VB.TextBox` | Text: 500000; ToolTipText: Brings the sun up, overriding day/night cycles if the sim energy falls below this value. | L=2640, T=1920, W=1395, H=285 | TabIndex=7, Alignment=1 |
| `Frame1 / Frame2 / SunDownUpDn` | `ComCtl2.UpDown` | ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=4080, T=1440, W=255, H=285 | Enabled=True, Value=100, TabIndex=11 |
| `Frame1 / Frame2 / SunUpUpDn` | `ComCtl2.UpDown` | ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=4080, T=1920, W=255, H=285 | Enabled=True, Value=100, TabIndex=12 |
| `Frame1 / Frame2 / Label1` | `VB.Label` | Caption: When Threshold reached: | L=240, T=360, W=2295, H=255 | TabIndex=14 |
| `Frame1 / DNLength` | `VB.TextBox` | Text: 1000000; ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=3000, T=360, W=1035, H=285 | TabIndex=3, Alignment=1 |
| `Frame1 / DNCheck` | `VB.CheckBox` | Caption: Enable Day Cycles; ToolTipText: A cycle of light and darkness. Veggies are not fed or repopulated during night time. | L=240, T=360, W=1635, H=255 | TabIndex=2 |
| `Frame1 / DNCycleUpDn` | `ComCtl2.UpDown` | ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=4080, T=360, W=255, H=285 | Enabled=True, Value=100, TabIndex=4 |
| `Frame1 / Label40` | `VB.Label` | Caption: Period; ToolTipText: Set the length of day and night in game cycles. The value entered here represents one full cycle of both. | L=2400, T=360, W=555, H=195 | TabIndex=5 |
| `OKButton` | `VB.CommandButton` | Caption: OK | L=2160, T=4560, W=1215, H=375 | TabIndex=0 |

## Shapes (`ObstacleForm`)

Source: [`ObstacleForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/ObstacleForm.frm)  
Class: `VB.Form`. Client size: `6945 × 6300` twips. Border style: `default`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Frame5` | `VB.Frame` | Caption: Physicality | L=5160, T=2400, W=1695, H=2295 | TabIndex=33 |
| `Frame5 / ShapesVisableCheck` | `VB.CheckBox` | Caption: Bots can see shapes | L=120, T=360, W=1455, H=495 | TabIndex=36 |
| `Frame5 / ShapesAbsorbShotsCheck` | `VB.CheckBox` | Caption: Shapes absorb shots | L=120, T=1440, W=1455, H=495 | TabIndex=35 |
| `Frame5 / ShapesSeeThroughCheck` | `VB.CheckBox` | Caption: Bots can see through shapes | L=120, T=960, W=1455, H=495 | TabIndex=34 |
| `HorizontalDriftCheck` | `VB.CheckBox` | Caption: Allow Horizontal Drift | L=600, T=2640, W=1815, H=255 | TabIndex=31 |
| `Maze` | `VB.Frame` | Caption: Maze Properties | L=120, T=4080, W=4935, H=2055 | TabIndex=22 |
| `Maze / MazeWidthSlider` | `MSComctlLib.Slider` | — | L=1200, T=480, W=3615, H=555 | Value=300, TabIndex=23 |
| `Maze / WallThicknessSlider` | `MSComctlLib.Slider` | — | L=1200, T=1320, W=3615, H=555 | Value=300, TabIndex=27 |
| `Maze / Label15` | `VB.Label` | Caption: Thick | L=4200, T=1080, W=495, H=255 | TabIndex=30 |
| `Maze / Label14` | `VB.Label` | Caption: Thin | L=1320, T=1080, W=735, H=255 | TabIndex=29 |
| `Maze / Label13` | `VB.Label` | Caption: Wall Thickness | L=360, T=1320, W=735, H=495 | TabIndex=28 |
| `Maze / Label12` | `VB.Label` | Caption: Corridor Width | L=360, T=480, W=735, H=495 | TabIndex=26 |
| `Maze / Label11` | `VB.Label` | Caption: Wide | L=4320, T=240, W=375, H=255 | TabIndex=25 |
| `Maze / Label10` | `VB.Label` | Caption: Narrow | L=1320, T=240, W=735, H=255 | TabIndex=24 |
| `Frame4` | `VB.Frame` | Caption: Movement | L=120, T=2400, W=4935, H=1575 | TabIndex=17 |
| `Frame4 / VerticalDriftCheck` | `VB.CheckBox` | Caption: Allow Vertical Drift | L=2760, T=240, W=1815, H=255 | TabIndex=32 |
| `Frame4 / DriftRateSlider` | `MSComctlLib.Slider` | — | L=1200, T=840, W=3615, H=555 | Value=1, TabIndex=18 |
| `Frame4 / Label9` | `VB.Label` | Caption: Fast | L=4440, T=600, W=375, H=255 | TabIndex=21 |
| `Frame4 / Label8` | `VB.Label` | Caption: Slow | L=1200, T=600, W=495, H=255 | TabIndex=20 |
| `Frame4 / Label7` | `VB.Label` | Caption: Velocity | L=120, T=840, W=855, H=255 | TabIndex=19 |
| `Frame3` | `VB.Frame` | Caption: Appearance | L=5160, T=1200, W=1695, H=1095 | TabIndex=6 |
| `Frame3 / OpaqueOption` | `VB.OptionButton` | Caption: Opaque | L=240, T=600, W=1215, H=375 | TabIndex=8 |
| `Frame3 / TransparentOption` | `VB.OptionButton` | Caption: Transparent | L=240, T=240, W=1215, H=375 | TabIndex=7 |
| `Frame2` | `VB.Frame` | Caption: Color | L=5160, T=120, W=1695, H=1095 | TabIndex=3 |
| `Frame2 / BlackColorOption` | `VB.OptionButton` | Caption: Black | L=240, T=600, W=1215, H=375 | TabIndex=5 |
| `Frame2 / RandomColorOption` | `VB.OptionButton` | Caption: Random | L=240, T=240, W=1215, H=375 | TabIndex=4 |
| `MakeShape` | `VB.CommandButton` | Caption: Add Shape | L=5400, T=5280, W=1215, H=375 | TabIndex=2 |
| `Frame1` | `VB.Frame` | Caption: Dimensions | L=120, T=120, W=4935, H=2175 | TabIndex=1 |
| `Frame1 / HeightSlider` | `MSComctlLib.Slider` | — | L=1200, T=480, W=3615, H=555 | Value=1, TabIndex=12 |
| `Frame1 / WidthSlider` | `MSComctlLib.Slider` | — | L=1200, T=1440, W=3615, H=555 | Value=1, TabIndex=13 |
| `Frame1 / Label6` | `VB.Label` | Caption: Fat | L=4440, T=1200, W=255, H=255 | TabIndex=16 |
| `Frame1 / Label5` | `VB.Label` | Caption: Skinny | L=1200, T=1200, W=615, H=255 | TabIndex=15 |
| `Frame1 / Label4` | `VB.Label` | Caption: Width | L=360, T=1560, W=615, H=255 | TabIndex=14 |
| `Frame1 / Label3` | `VB.Label` | Caption: Tall | L=4320, T=240, W=375, H=255 | TabIndex=11 |
| `Frame1 / Label2` | `VB.Label` | Caption: Short | L=1200, T=240, W=615, H=255 | TabIndex=10 |
| `Frame1 / Label1` | `VB.Label` | Caption: Height | L=360, T=600, W=615, H=255 | TabIndex=9 |
| `OK` | `VB.CommandButton` | Caption: OK | L=5400, T=5760, W=1215, H=375 | TabIndex=0 |

## New Teleporter (`TeleportForm`)

Source: [`TeleportForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/TeleportForm.frm)  
Class: `VB.Form`. Client size: `8790 × 6420` twips. Border style: `4`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Frame5` | `VB.Frame` | Caption: Internet Options | L=4800, T=2880, W=3855, H=1935 | TabIndex=32 |
| `Frame5 / intOutText` | `VB.TextBox` | — | L=240, T=1440, W=3375, H=285 | TabIndex=34 |
| `Frame5 / intInText` | `VB.TextBox` | — | L=240, T=720, W=3375, H=285 | TabIndex=33 |
| `Frame5 / inboundLabel` | `VB.Label` | Caption: Inbound Path | L=240, T=480, W=1095, H=255 | TabIndex=36 |
| `Frame5 / outboundLabel` | `VB.Label` | Caption: Outbound Path | L=240, T=1200, W=1335, H=255 | TabIndex=35 |
| `Frame4` | `VB.Frame` | Caption: Statistics | L=120, T=3840, W=4575, H=975 | TabIndex=12 |
| `Frame4 / NumTeleported` | `VB.Label` | Caption: 0 | L=1800, T=360, W=2055, H=255 | TabIndex=14 |
| `Frame4 / Label3` | `VB.Label` | Caption: Total Teleported: | L=240, T=360, W=1215, H=255 | TabIndex=13 |
| `Frame3` | `VB.Frame` | Caption: Type | L=120, T=720, W=4575, H=3015 | TabIndex=7 |
| `Frame3 / TeleportOption` | `VB.OptionButton` | Caption: Internet | L=600, T=960, W=975, H=255 | Index=3, Enabled=0, TabIndex=31 |
| `Frame3 / BackFlowText` | `VB.TextBox` | Text: 10 | L=2160, T=2280, W=735, H=285 | TabIndex=30 |
| `Frame3 / TeleportVeggiesCheck` | `VB.CheckBox` | Caption: Teleport Autotrophs | L=2400, T=600, W=1695, H=195 | TabIndex=23 |
| `Frame3 / TeleportCorpsesCheck` | `VB.CheckBox` | Caption: Teleport Corpses | L=2400, T=840, W=1575, H=195 | TabIndex=22 |
| `Frame3 / TeleportHeterotrophsCheck` | `VB.CheckBox` | Caption: Teleport Heterotrophs | L=2400, T=360, W=1935, H=195 | TabIndex=21 |
| `Frame3 / BotsPerPoll` | `VB.TextBox` | Text: 10 | L=2160, T=1920, W=735, H=285 | TabIndex=19 |
| `Frame3 / InboundCycleCheck` | `VB.TextBox` | Text: 10 | L=2160, T=1560, W=735, H=285 | TabIndex=16 |
| `Frame3 / TeleportOption` | `VB.OptionButton` | Caption: Inbound | L=600, T=720, W=975, H=255 | Index=0, TabIndex=10 |
| `Frame3 / TeleportOption` | `VB.OptionButton` | Caption: Outbound | L=600, T=480, W=1095, H=255 | Index=1, TabIndex=9 |
| `Frame3 / TeleportOption` | `VB.OptionButton` | Caption: Intrasim | L=600, T=240, W=975, H=255 | Index=2, TabIndex=8 |
| `Frame3 / Label5` | `VB.Label` | Caption: Backflow Limit | L=960, T=2280, W=1095, H=255 | TabIndex=29 |
| `Frame3 / Label4` | `VB.Label` | Caption: bots | L=3000, T=2280, W=1335, H=255 | TabIndex=28 |
| `Frame3 / InboundLabel4` | `VB.Label` | Caption: bots in at a time | L=3000, T=1920, W=1455, H=255 | TabIndex=20 |
| `Frame3 / InboundLabel3` | `VB.Label` | Caption: Teleport max of  | L=960, T=1920, W=1215, H=255 | TabIndex=18 |
| `Frame3 / InboundLabel2` | `VB.Label` | Caption: cycles | L=3000, T=1560, W=615, H=255 | TabIndex=17 |
| `Frame3 / InboundLabel1` | `VB.Label` | Caption: Check every | L=1080, T=1560, W=975, H=255 | TabIndex=15 |
| `Frame1` | `VB.Frame` | Caption: Properties | L=4800, T=720, W=3855, H=1935 | TabIndex=5 |
| `Frame1 / Frame2` | `VB.Frame` | Caption: Size | L=240, T=960, W=3495, H=855 | TabIndex=24 |
| `Frame1 / Frame2 / TeleporterSizeSlider` | `MSComctlLib.Slider` | — | L=240, T=480, W=3135, H=315 | Value=100, TabIndex=25 |
| `Frame1 / Frame2 / Label10` | `VB.Label` | Caption: Small | L=240, T=240, W=735, H=255 | TabIndex=27 |
| `Frame1 / Frame2 / Label11` | `VB.Label` | Caption: Large | L=2880, T=240, W=495, H=255 | TabIndex=26 |
| `Frame1 / RespectShapesCheck` | `VB.CheckBox` | Caption: Respect Shapes | L=480, T=600, W=1575, H=195 | Enabled=0, TabIndex=11 |
| `Frame1 / FixedCheck` | `VB.CheckBox` | Caption: Fixed | L=480, T=360, W=1095, H=195 | TabIndex=6 |
| `CancelButton` | `VB.CommandButton` | Caption: Cancel | L=5280, T=5760, W=1215, H=375 | TabIndex=3 |
| `OKButton` | `VB.CommandButton` | Caption: OK | L=7080, T=5760, W=1215, H=375 | TabIndex=2 |
| `NetworkPath` | `VB.TextBox` | Text: path; ToolTipText: The folder or network share to use to exchange bots | L=1320, T=240, W=2535, H=285 | TabIndex=0 |
| `Label42` | `VB.Label` | Caption: $"TeleportForm.frx":0000 | L=120, T=4920, W=8535, H=735 | TabIndex=37 |
| `Label2` | `VB.Label` | Caption: e.g.   c:\db   or  \\server\share | L=4080, T=240, W=2295, H=255 | TabIndex=4 |
| `Label1` | `VB.Label` | Caption: File Path: | L=240, T=240, W=1215, H=255 | TabIndex=1 |

## Global Settings (`frmGset`)

Source: [`frmGset.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmGset.frm)  
Class: `VB.Form`. Client size: `10770 × 5700` twips. Border style: `4`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `btnOK` | `VB.CommandButton` | Caption: OK | L=8160, T=5280, W=1215, H=375 | TabIndex=2 |
| `btnCancel` | `VB.CommandButton` | Caption: Cancel | L=9480, T=5280, W=1215, H=375 | TabIndex=1 |
| `tb` | `TabDlg.SSTab` | — | L=0, T=0, W=10755, H=5175 | Style=1, TabIndex=0 |
| `tb / chkIntRnd` | `VB.CheckBox` | Caption: Use Internet Pictures to seed Randomizer | L=-67920, T=1920, W=3375, H=255 | TabIndex=93 |
| `tb / chkEpiGene` | `VB.CheckBox` | Caption: Save epigenetic memory as a temporary gene when saving robot DNA to file.        Warning: This uses delgene. | L=-69480, T=4560, W=4815, H=375 | TabIndex=92 |
| `tb / chkFilter` | `VB.CheckBox` | Caption: Filter by specific robot | L=-72000, T=2280, W=5415, H=195 | TabIndex=91 |
| `tb / Command1` | `VB.CommandButton` | Caption: League Restrictions | L=-68880, T=4080, W=2175, H=375 | TabIndex=87 |
| `tb / btnEvoRES` | `VB.CommandButton` | Caption: Evolution Restrictions | L=240, T=3480, W=2175, H=375 | TabIndex=86 |
| `tb / chkAddarob` | `VB.CheckBox` | Caption: Add a single robot to existing league | L=-70200, T=1920, W=5415, H=195 | Visible=0, TabIndex=85 |
| `tb / chkSurvivalEco` | `VB.CheckBox` | Caption: Eco Survival Mode | L=3960, T=4080, W=2295, H=255 | TabIndex=67 |
| `tb / chkZBmode` | `VB.CheckBox` | Caption: Zerobot Mode | L=2400, T=4080, W=2295, H=255 | TabIndex=84 |
| `tb / ffmZeroBot` | `VB.Frame` | Caption: Zerobot settings | L=7320, T=600, W=2175, H=1095 | Visible=0, TabIndex=81 |
| `tb / ffmZeroBot / txtZlength` | `VB.TextBox` | Text: 128 | L=720, T=480, W=1215, H=375 | TabIndex=82 |
| `tb / ffmZeroBot / lblZlength` | `VB.Label` | Caption: Initial length: | L=120, T=360, W=615, H=495 | TabIndex=83 |
| `tb / chkSurvivalSimple` | `VB.CheckBox` | Caption: Simple Survival Mode | L=360, T=4080, W=2295, H=255 | TabIndex=80 |
| `tb / ffmSurvival` | `VB.Frame` | Caption: Survival Mode Settings | L=240, T=600, W=6855, H=2775 | Visible=0, TabIndex=68 |
| `tb / ffmSurvival / chkNormSize` | `VB.CheckBox` | Caption: Dynamically normalize DNA size | L=120, T=2400, W=2655, H=255 | Visible=0, TabIndex=76 |
| `tb / ffmSurvival / chkIM` | `VB.CheckBox` | Caption: Start with IM on | L=3000, T=2400, W=1935, H=255 | Visible=0, TabIndex=75 |
| `tb / ffmSurvival / txtCycSM` | `VB.TextBox` | Text: 1500 | L=2520, T=1080, W=855, H=375 | TabIndex=74 |
| `tb / ffmSurvival / chkShowGraphs` | `VB.CheckBox` | Caption: Auto run Population, Mutation, Total energy graphs with saving enabled | L=120, T=2040, W=5895, H=375 | TabIndex=73 |
| `tb / ffmSurvival / txtRob` | `VB.TextBox` | — | L=120, T=480, W=5415, H=405 | TabIndex=72 |
| `tb / ffmSurvival / btnBrowseRob` | `VB.CommandButton` | Caption: Browse... | L=5640, T=480, W=1095, H=375 | TabIndex=71 |
| `tb / ffmSurvival / txtLFOR` | `VB.TextBox` | Text: 2 | L=2760, T=1560, W=945, H=375 | TabIndex=70 |
| `tb / ffmSurvival / btnHelp` | `VB.CommandButton` | Caption: ? | L=4320, T=1650, W=255, H=255 | TabIndex=69 |
| `tb / ffmSurvival / lblCycSM` | `VB.Label` | Caption: Initial reintroduction on/off length  XXXXXXXX cycles | L=120, T=1150, W=4215, H=255 | TabIndex=79 |
| `tb / ffmSurvival / lblRob` | `VB.Label` | Caption: Source Robot | L=120, T=240, W=1935, H=375 | TabIndex=78 |
| `tb / ffmSurvival / lblLFOR` | `VB.Label` | Caption: Initial population oscillation reduction   XXXXXXXX units | L=120, T=1635, W=5535, H=255 | TabIndex=77 |
| `tb / ffmDisqualification` | `VB.Frame` | Caption: Disqualification Rules on Contest | L=-72120, T=3720, W=3015, H=975 | TabIndex=63 |
| `tb / ffmDisqualification / optDisqua` | `VB.OptionButton` | Caption: F3; ToolTipText: Look for Disqualifications.txt in your main directory. | L=1920, T=480, W=735, H=255 | Index=2, TabIndex=66 |
| `tb / ffmDisqualification / optDisqua` | `VB.OptionButton` | Caption: F2; ToolTipText: Look for Disqualifications.txt in your main directory. | L=1200, T=480, W=615, H=255 | Index=1, TabIndex=65 |
| `tb / ffmDisqualification / optDisqua` | `VB.OptionButton` | Caption: None | L=240, T=480, W=855, H=255 | Index=0, Value=True, TabIndex=64 |
| `tb / ffmInitChlr` | `VB.Frame` | Caption: Advanced Chloroplast Options | L=-70080, T=3480, W=5655, H=975 | TabIndex=60 |
| `tb / ffmInitChlr / txtStartChlr` | `VB.TextBox` | Text: 0 | L=2400, T=360, W=1455, H=375 | TabIndex=61 |
| `tb / ffmInitChlr / lblStartChlr` | `VB.Label` | Caption: Start Repopulating Robots with  XXXXXXXXXXXXX  Chloroplasts | L=120, T=435, W=5295, H=255 | TabIndex=62 |
| `tb / ffmFudge` | `VB.Frame` | Caption: Fudging on F1 Contest | L=-72120, T=2640, W=5535, H=975 | TabIndex=56 |
| `tb / ffmFudge / optFudging` | `VB.OptionButton` | Caption: All possible recognition methods | L=2760, T=480, W=2655, H=255 | Index=2, TabIndex=59 |
| `tb / ffmFudge / optFudging` | `VB.OptionButton` | Caption: Eyes only | L=1440, T=480, W=1095, H=255 | Index=1, TabIndex=58 |
| `tb / ffmFudge / optFudging` | `VB.OptionButton` | Caption: None | L=240, T=480, W=1095, H=255 | Index=0, Value=True, TabIndex=57 |
| `tb / chkStepladder` | `VB.CheckBox` | Caption: Stepladder league     (click for more options) | L=-72000, T=1920, W=5415, H=195 | TabIndex=55 |
| `tb / txtSourceDir` | `VB.TextBox` | Text: C:\ | L=-72000, T=960, W=5415, H=405 | Visible=0, TabIndex=54 |
| `tb / chkTournament` | `VB.CheckBox` | Caption: Tournament league | L=-72000, T=1560, W=1815, H=195 | TabIndex=52 |
| `tb / ffmSunMut` | `VB.Frame` | Caption: Sunline Mutations | L=-74160, T=1280, W=9135, H=3775 | TabIndex=30 |
| `tb / ffmSunMut / sldMain` | `MSComctlLib.Slider` | ToolTipText: Probability Delta2 Chance | L=7080, T=960, W=1935, H=285 | Visible=0, TabIndex=50 |
| `tb / ffmSunMut / txtWTC` | `VB.TextBox` | ToolTipText: Oscillation between type and value | L=6960, T=1920, W=615, H=285 | TabIndex=48 |
| `tb / ffmSunMut / txtMxDnalen` | `VB.TextBox` | — | L=6240, T=2760, W=615, H=285 | TabIndex=46 |
| `tb / ffmSunMut / txtDnalen` | `VB.TextBox` | — | L=3600, T=2760, W=615, H=285 | TabIndex=44 |
| `tb / ffmSunMut / chkNorm` | `VB.CheckBox` | Caption: Normalize default mutation rates and slowest possible rate based on DNA length | L=120, T=2400, W=6015, H=255 | TabIndex=43 |
| `tb / ffmSunMut / txtPMinter` | `VB.TextBox` | — | L=4320, T=1920, W=615, H=285 | TabIndex=42 |
| `tb / ffmSunMut / txtDevLn` | `VB.TextBox` | — | L=6480, T=1440, W=615, H=285 | Visible=0, TabIndex=40 |
| `tb / ffmSunMut / txtDevExp` | `VB.TextBox` | — | L=5280, T=1440, W=615, H=285 | Visible=0, TabIndex=38 |
| `tb / ffmSunMut / txtMainLn` | `VB.TextBox` | — | L=6240, T=960, W=615, H=285 | Visible=0, TabIndex=37 |
| `tb / ffmSunMut / txtMainExp` | `VB.TextBox` | — | L=4995, T=960, W=615, H=285 | Visible=0, TabIndex=35 |
| `tb / ffmSunMut / chkDelta2` | `VB.CheckBox` | Caption: Enable Delta2 Mutations | L=120, T=600, W=2055, H=255 | TabIndex=34 |
| `tb / ffmSunMut / picIcon` | `VB.PictureBox` | — | L=120, T=3120, W=540, H=540 | TabIndex=33 |
| `tb / ffmSunMut / chkSunbelt` | `VB.CheckBox` | Caption: Enable Point2, CopyError2, Amplification, and Translocation Mutations | L=120, T=300, W=5295, H=255 | TabIndex=32 |
| `tb / ffmSunMut / sldDev` | `MSComctlLib.Slider` | ToolTipText: Mean/Stddev Delta2 Chance | L=7080, T=1440, W=1935, H=285 | Visible=0, TabIndex=51 |
| `tb / ffmSunMut / lblD2` | `VB.Label` | Caption: Click here for some really advanced delta2 settings. | L=2640, T=1200, W=6135, H=255 | BorderStyle=1, TabIndex=88, Alignment=2 |
| `tb / ffmSunMut / lblChance` | `VB.Label` | Caption: Chance of mutation % | L=7200, T=720, W=1575, H=255 | Visible=0, TabIndex=49 |
| `tb / ffmSunMut / lblWTC` | `VB.Label` | Caption: Delta2 for value(s) of what to change:     ±; ToolTipText: Oscillation between type and value | L=5280, T=1800, W=1575, H=495 | TabIndex=47 |
| `tb / ffmSunMut / lblDnalen` | `VB.Label` | Caption: DNA length *    XXXX     Slowest rate DNA length *  | L=2520, T=2760, W=4455, H=255 | TabIndex=45 |
| `tb / ffmSunMut / lblPMDelta2` | `VB.Label` | Caption: Delta2 cycle interval for Point mutations: | L=2520, T=1800, W=1695, H=495 | TabIndex=41 |
| `tb / ffmSunMut / lblMmean` | `VB.Label` | Caption: Mean/Stddev:    Multiply by (10 ^ ± 1/   XXXX   ) ± Liner | L=2520, T=1440, W=4335, H=375 | Visible=0, TabIndex=39 |
| `tb / ffmSunMut / lblMmain` | `VB.Label` | Caption: Probablity:    Multiply by (10 ^ ± 1/   XXXX   ) ± Liner | L=2520, T=960, W=3975, H=375 | Visible=0, TabIndex=36 |
| `tb / ffmSunMut / lblExplMut` | `VB.Label` | Caption: $"frmGset.frx":0CB2 | L=720, T=3100, W=8175, H=600 | TabIndex=31 |
| `tb / ffmEpiReset` | `VB.Frame` | Caption: Epigenetic Reset | L=-73440, T=460, W=7575, H=735 | TabIndex=23 |
| `tb / ffmEpiReset / txtOP` | `VB.TextBox` | Text: 17; ToolTipText: This is how much exponential amplified mutations a robot must have in order to trigger an epigenetic memory reset. | L=6840, T=240, W=615, H=375 | TabIndex=27 |
| `tb / ffmEpiReset / txtMEmp` | `VB.TextBox` | Text: 1.3; ToolTipText: $"frmGset.frx":0DC4 | L=4920, T=240, W=615, H=375 | TabIndex=25 |
| `tb / ffmEpiReset / chkEpiReset` | `VB.CheckBox` | Caption: Periodically reset Epigenetic memory | L=120, T=300, W=2895, H=255 | TabIndex=24 |
| `tb / ffmEpiReset / lblOP` | `VB.Label` | Caption: overload point: | L=5640, T=240, W=1215, H=255 | TabIndex=28 |
| `tb / ffmEpiReset / lblMEmp` | `VB.Label` | Caption: mutation amplification: | L=3240, T=240, W=1695, H=255 | TabIndex=26 |
| `tb / ffmFBSBO` | `VB.Frame` | Caption: Find Best Settings base on: | L=-70080, T=2280, W=5655, H=915 | TabIndex=16 |
| `tb / ffmFBSBO / sldFindBest` | `MSComctlLib.Slider` | — | L=1320, T=240, W=2955, H=570 | TabIndex=17, TabStop=0 |
| `tb / ffmFBSBO / lblTP` | `VB.Label` | Caption: Total Population | L=120, T=420, W=1215, H=195 | TabIndex=19 |
| `tb / ffmFBSBO / lblIE` | `VB.Label` | Caption: Invested Energy | L=4300, T=420, W=1200, H=195 | TabIndex=18 |
| `tb / chkSafeMode` | `VB.CheckBox` | Caption: Use Safe Mode | L=-69840, T=1920, W=1575, H=255 | TabIndex=15 |
| `tb / ffmMainDir` | `VB.Frame` | Caption: Main Directory | L=-70080, T=480, W=5655, H=1215 | TabIndex=12 |
| `tb / ffmMainDir / txtCD` | `VB.TextBox` | Text: Text1 | L=120, T=720, W=5415, H=375 | TabIndex=14 |
| `tb / ffmMainDir / chkUseCD` | `VB.CheckBox` | Caption: Change Directory | L=120, T=360, W=1575, H=195 | TabIndex=13 |
| `tb / Frame1` | `VB.Frame` | Caption: Randomization | L=-74880, T=3960, W=4695, H=1095 | TabIndex=9 |
| `tb / Frame1 / chkchseedloadsim` | `VB.CheckBox` | Caption: Generate new seed when you click 'Load Simulation' | L=240, T=600, W=4275, H=375 | TabIndex=11 |
| `tb / Frame1 / chkchseedstartnew` | `VB.CheckBox` | Caption: Generate new seed when you click 'Start new' | L=240, T=240, W=4275, H=375 | TabIndex=10 |
| `tb / ffmCheatin` | `VB.Frame` | Caption: Cheating Prevention | L=-74880, T=2375, W=4695, H=1575 | TabIndex=5 |
| `tb / ffmCheatin / chkGreedy` | `VB.CheckBox` | Caption: Kill robots that are excessively greedy to there kids, using them to dump there energy. | L=240, T=960, W=3615, H=375 | TabIndex=8 |
| `tb / ffmCheatin / txtBodyFix` | `VB.TextBox` | Text: 32100 | L=1200, T=480, W=1455, H=375 | TabIndex=7 |
| `tb / ffmCheatin / CheatinLab` | `VB.Label` | Caption: Kill robots that have more then this amound of body to prevent BigBerthas: | L=120, T=240, W=4455, H=615 | TabIndex=6 |
| `tb / ffmUI` | `VB.Frame` | Caption: UI Settings | L=-74880, T=360, W=4695, H=2000 | TabIndex=3 |
| `tb / ffmUI / chkHide` | `VB.CheckBox` | Caption: Hide Darwinbots on restart mode in system tray | L=240, T=1650, W=3975, H=255 | TabIndex=90 |
| `tb / ffmUI / chkScreenRatio` | `VB.CheckBox` | Caption: Fix Screen Ratio when simulation starts | L=240, T=270, W=3075, H=255 | TabIndex=4 |
| `tb / ffmUI / chkGraphUp` | `VB.CheckBox` | Caption: Automatically assume to keep updating graphs | L=240, T=1365, W=3975, H=255 | TabIndex=89 |
| `tb / ffmUI / chkNoVid` | `VB.CheckBox` | Caption: Turn off Video when simulation starts | L=240, T=1080, W=3495, H=255 | TabIndex=22 |
| `tb / ffmUI / chkNoBoyMsg` | `VB.CheckBox` | Caption: Don't display Buoyancy Warning | L=240, T=810, W=2775, H=255 | TabIndex=21 |
| `tb / ffmUI / chkOldColor` | `VB.CheckBox` | Caption: Use old simulation colors | L=240, T=480, W=3075, H=375 | TabIndex=20 |
| `tb / lblSource` | `VB.Label` | Caption: Source Directory | L=-72000, T=720, W=1935, H=375 | Visible=0, TabIndex=53 |
| `lbl` | `VB.Label` | Caption: Note: To reset all values delete global.gset file from your main directory. | L=120, T=5400, W=7455, H=255 | TabIndex=29 |

## RGB Memory Monitor Settings (`frmMonitorSet`)

Source: [`frmMonitorSet.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmMonitorSet.frm)  
Class: `VB.Form`. Client size: `5895 × 3255` twips. Border style: `5`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `CommonDialog1` | `MSComDlg.CommonDialog` | — | L=120, T=2760 | — |
| `Command2` | `VB.CommandButton` | Caption: Save Preset... | L=1560, T=2760, W=1335, H=375 | TabIndex=27 |
| `Command1` | `VB.CommandButton` | Caption: Load Preset... | L=120, T=2760, W=1335, H=375 | TabIndex=26 |
| `ffmRed` | `VB.Frame` | Caption: Blue | L=3960, T=120, W=1815, H=2535 | Index=2, TabIndex=18 |
| `ffmRed / txtMem` | `VB.TextBox` | Text: 1 | L=120, T=480, W=855, H=285 | Index=2, TabIndex=24 |
| `ffmRed / ffmRange` | `VB.Frame` | Caption: Active Range | L=120, T=840, W=1575, H=1575 | Index=2, TabIndex=19 |
| `ffmRed / ffmRange / txtCeil` | `VB.TextBox` | Text: 32000 | L=120, T=1080, W=855, H=285 | Index=2, TabIndex=21 |
| `ffmRed / ffmRange / txtFloor` | `VB.TextBox` | Text: 0 | L=120, T=480, W=855, H=285 | Index=2, TabIndex=20 |
| `ffmRed / ffmRange / Label3` | `VB.Label` | Caption: Ceiling: | L=120, T=840, W=615, H=255 | Index=2, TabIndex=23 |
| `ffmRed / ffmRange / Label2` | `VB.Label` | Caption: Floor: | L=120, T=240, W=615, H=255 | Index=2, TabIndex=22 |
| `ffmRed / Label1` | `VB.Label` | Caption: Memory location: | L=120, T=240, W=1215, H=255 | Index=2, TabIndex=25 |
| `ffmRed` | `VB.Frame` | Caption: Green | L=2040, T=120, W=1815, H=2535 | Index=1, TabIndex=10 |
| `ffmRed / ffmRange` | `VB.Frame` | Caption: Active Range | L=120, T=840, W=1575, H=1575 | Index=1, TabIndex=12 |
| `ffmRed / ffmRange / txtFloor` | `VB.TextBox` | Text: 0 | L=120, T=480, W=855, H=285 | Index=1, TabIndex=14 |
| `ffmRed / ffmRange / txtCeil` | `VB.TextBox` | Text: 32000 | L=120, T=1080, W=855, H=285 | Index=1, TabIndex=13 |
| `ffmRed / ffmRange / Label2` | `VB.Label` | Caption: Floor: | L=120, T=240, W=615, H=255 | Index=1, TabIndex=16 |
| `ffmRed / ffmRange / Label3` | `VB.Label` | Caption: Ceiling: | L=120, T=840, W=615, H=255 | Index=1, TabIndex=15 |
| `ffmRed / txtMem` | `VB.TextBox` | Text: 1 | L=120, T=480, W=855, H=285 | Index=1, TabIndex=11 |
| `ffmRed / Label1` | `VB.Label` | Caption: Memory location: | L=120, T=240, W=1215, H=255 | Index=1, TabIndex=17 |
| `ffmRed` | `VB.Frame` | Caption: Red | L=120, T=120, W=1815, H=2535 | Index=0, TabIndex=2 |
| `ffmRed / txtMem` | `VB.TextBox` | Text: 1 | L=120, T=480, W=855, H=285 | Index=0, TabIndex=5 |
| `ffmRed / ffmRange` | `VB.Frame` | Caption: Active Range | L=120, T=840, W=1575, H=1575 | Index=0, TabIndex=4 |
| `ffmRed / ffmRange / txtCeil` | `VB.TextBox` | Text: 32000 | L=120, T=1080, W=855, H=285 | Index=0, TabIndex=9 |
| `ffmRed / ffmRange / txtFloor` | `VB.TextBox` | Text: 0 | L=120, T=480, W=855, H=285 | Index=0, TabIndex=7 |
| `ffmRed / ffmRange / Label3` | `VB.Label` | Caption: Ceiling: | L=120, T=840, W=615, H=255 | Index=0, TabIndex=8 |
| `ffmRed / ffmRange / Label2` | `VB.Label` | Caption: Floor: | L=120, T=240, W=615, H=255 | Index=0, TabIndex=6 |
| `ffmRed / Label1` | `VB.Label` | Caption: Memory location: | L=120, T=240, W=1215, H=255 | Index=0, TabIndex=3 |
| `btnCancel` | `VB.CommandButton` | Caption: Cancel | L=4560, T=2760, W=1215, H=375 | TabIndex=1 |
| `btnOK` | `VB.CommandButton` | Caption: OK | L=3240, T=2760, W=1215, H=375 | TabIndex=0 |

## Player Bot Mode Settings (`frmPBMode`)

Source: [`frmPBMode.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmPBMode.frm)  
Class: `VB.Form`. Client size: `5865 × 3420` twips. Border style: `4`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `ffmSett` | `VB.Frame` | — | L=120, T=0, W=5655, H=3375 | TabIndex=0 |
| `ffmSett / btnDel` | `VB.CommandButton` | Caption: Delete | L=1200, T=2880, W=975, H=375 | TabIndex=6 |
| `ffmSett / btnAdd` | `VB.CommandButton` | Caption: Add... | L=120, T=2880, W=975, H=375 | TabIndex=5 |
| `ffmSett / lstkey` | `VB.ListBox` | — | L=120, T=240, W=5415, H=2460 | TabIndex=4 |
| `ffmSett / btnLoad` | `VB.CommandButton` | Caption: Load Preset... | L=2760, T=2880, W=1335, H=375 | TabIndex=3 |
| `ffmSett / btnSave` | `VB.CommandButton` | Caption: Save Preset... | L=4200, T=2880, W=1335, H=375 | TabIndex=2 |
| `CommonDialog1` | `MSComDlg.CommonDialog` | — | L=0, T=0 | — |
| `lblPress` | `VB.Label` | Caption: Press any key... | L=0, T=0, W=5895, H=495 | Visible=0, TabIndex=1 |

## Restriction Options (`frmRestriOps`)

Source: [`frmRestriOps.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmRestriOps.frm)  
Class: `VB.Form`. Client size: `7290 × 4320` twips. Border style: `4`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `btnLoad` | `VB.CommandButton` | Caption: Load Preset... | L=120, T=3840, W=1335, H=375 | TabIndex=27 |
| `btnSave` | `VB.CommandButton` | Caption: Save Preset... | L=1560, T=3840, W=1335, H=375 | TabIndex=26 |
| `btnOK` | `VB.CommandButton` | Caption: Apply | L=6000, T=3840, W=1215, H=375 | TabIndex=2 |
| `ffmNChlr` | `VB.Frame` | Caption: For Non-Repupulating Robots | L=120, T=0, W=3495, H=3735 | TabIndex=1 |
| `ffmNChlr / ffmP` | `VB.Frame` | Caption: Property Applied Restrictions | L=120, T=1200, W=3255, H=2415 | TabIndex=16 |
| `ffmNChlr / ffmP / MutEnabledCheck` | `VB.CheckBox` | Caption: Disable Mutations; ToolTipText: Disables voluntary movement for this species | L=120, T=1715, W=2430, H=330 | TabIndex=17 |
| `ffmNChlr / ffmP / VirusImmuneCheck` | `VB.CheckBox` | Caption: Virus Immune; ToolTipText: Disables voluntary movement for this species | L=120, T=1475, W=2430, H=330 | TabIndex=18 |
| `ffmNChlr / ffmP / DisableMovementSysvarsCheck` | `VB.CheckBox` | Caption: Disable Voluntary Movement; ToolTipText: Disables voluntary movement for this species | L=120, T=1985, W=2430, H=330 | TabIndex=19 |
| `ffmNChlr / ffmP / DisableReproductionCheck` | `VB.CheckBox` | Caption: Disable Reproduction; ToolTipText: Disables voluntary movement for this species | L=120, T=1225, W=2430, H=330 | TabIndex=24 |
| `ffmNChlr / ffmP / DisableDNACheck` | `VB.CheckBox` | Caption: Disable DNA Execution; ToolTipText: Speeds up the simulation by turning off DNA execution for this species | L=120, T=995, W=2430, H=330 | TabIndex=20 |
| `ffmNChlr / ffmP / DisableVisionCheck` | `VB.CheckBox` | Caption: Disable Vision; ToolTipText: Speeds up the simulation by turning off vision for this species | L=120, T=745, W=2430, H=330 | TabIndex=21 |
| `ffmNChlr / ffmP / chkNoChlr` | `VB.CheckBox` | Caption: Disable Chloroplasts; ToolTipText: Feed automatically this type (only for vegetables) | L=120, T=320, W=2430, H=210 | TabIndex=23 |
| `ffmNChlr / ffmP / BlockSpec` | `VB.CheckBox` | Caption: Fixed in place | L=120, T=515, W=2430, H=330 | TabIndex=22 |
| `ffmNChlr / ffmKillNChlr` | `VB.Frame` | Caption: Fatal Restrictions | L=120, T=240, W=3255, H=975 | TabIndex=3 |
| `ffmNChlr / ffmKillNChlr / chkDQKillNChlr` | `VB.CheckBox` | Caption: Kill robot if it is disqualified from league | L=120, T=600, W=3015, H=255 | TabIndex=7 |
| `ffmNChlr / ffmKillNChlr / chkMBKillNChlr` | `VB.CheckBox` | Caption: Kill robot if robot not MB for 150 cycles; ToolTipText: For Multi-bots | L=120, T=240, W=3015, H=375 | TabIndex=5 |
| `ffmChlr` | `VB.Frame` | Caption: For Repupulating Robots | L=3720, T=0, W=3495, H=3375 | TabIndex=0 |
| `ffmChlr / ffmPChlr` | `VB.Frame` | Caption: Property Applied Restrictions | L=120, T=1200, W=3255, H=2055 | TabIndex=9 |
| `ffmChlr / ffmPChlr / DisableMovementSysvarsCheckVeg` | `VB.CheckBox` | Caption: Disable Voluntary Movement; ToolTipText: Disables voluntary movement for this species | L=120, T=1680, W=2550, H=330 | TabIndex=12 |
| `ffmChlr / ffmPChlr / MutEnabledCheckVeg` | `VB.CheckBox` | Caption: Disable Mutations; ToolTipText: Disables voluntary movement for this species | L=120, T=1440, W=2430, H=330 | TabIndex=10 |
| `ffmChlr / ffmPChlr / VirusImmuneCheckVeg` | `VB.CheckBox` | Caption: Virus Immune; ToolTipText: Disables voluntary movement for this species | L=120, T=1200, W=2430, H=330 | TabIndex=11 |
| `ffmChlr / ffmPChlr / DisableReproductionCheckChlr` | `VB.CheckBox` | Caption: Disable Reproduction; ToolTipText: Disables voluntary movement for this species | L=120, T=960, W=2430, H=330 | TabIndex=25 |
| `ffmChlr / ffmPChlr / DisableDNACheckVeg` | `VB.CheckBox` | Caption: Disable DNA Execution; ToolTipText: Speeds up the simulation by turning off DNA execution for this species | L=120, T=720, W=2430, H=330 | TabIndex=13 |
| `ffmChlr / ffmPChlr / DisableVisionCheckVeg` | `VB.CheckBox` | Caption: Disable Vision; ToolTipText: Speeds up the simulation by turning off vision for this species | L=120, T=480, W=2430, H=330 | TabIndex=14 |
| `ffmChlr / ffmPChlr / BlockSpecVeg` | `VB.CheckBox` | Caption: Fixed in place | L=120, T=240, W=2430, H=330 | TabIndex=15 |
| `ffmChlr / ffmkillChlr` | `VB.Frame` | Caption: Fatal Restrictions | L=120, T=240, W=3255, H=975 | TabIndex=4 |
| `ffmChlr / ffmkillChlr / chkDQKillChlr` | `VB.CheckBox` | Caption: Kill robot if it is disqualified from league | L=120, T=520, W=3015, H=375 | TabIndex=8 |
| `ffmChlr / ffmkillChlr / chkMBKillChlr` | `VB.CheckBox` | Caption: Kill robot if robot not MB for 150 cycles; ToolTipText: For Multi-bots | L=120, T=240, W=3015, H=375 | TabIndex=6 |
| `CommonDialog1` | `MSComDlg.CommonDialog` | — | L=0, T=0 | — |

## Eye Designer (`frmEYE`)

Source: [`frmEYE.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmEYE.frm)  
Class: `VB.Form`. Client size: `3975 × 5595` twips. Border style: `4`. Startup position: `3`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `CommonDialog1` | `MSComDlg.CommonDialog` | — | L=2880, T=2880 | — |
| `btnOut` | `VB.CommandButton` | Caption: Write DNA ... | L=120, T=3720, W=1335, H=375 | TabIndex=32 |
| `txtWth` | `VB.TextBox` | Text: 0 | L=1920, T=3240, W=975, H=285 | Index=8, TabIndex=31 |
| `txtWth` | `VB.TextBox` | Text: 0 | L=1920, T=2880, W=975, H=285 | Index=7, TabIndex=30 |
| `txtWth` | `VB.TextBox` | Text: 0 | L=1920, T=2520, W=975, H=285 | Index=6, TabIndex=29 |
| `txtWth` | `VB.TextBox` | Text: 0 | L=1920, T=2160, W=975, H=285 | Index=5, TabIndex=28 |
| `txtWth` | `VB.TextBox` | Text: 0 | L=1920, T=1800, W=975, H=285 | Index=4, TabIndex=27 |
| `txtWth` | `VB.TextBox` | Text: 0 | L=1920, T=1440, W=975, H=285 | Index=3, TabIndex=26 |
| `txtWth` | `VB.TextBox` | Text: 0 | L=1920, T=1080, W=975, H=285 | Index=2, TabIndex=25 |
| `txtWth` | `VB.TextBox` | Text: 0 | L=1920, T=720, W=975, H=285 | Index=1, TabIndex=24 |
| `txtWth` | `VB.TextBox` | Text: 0 | L=1920, T=360, W=975, H=285 | Index=0, TabIndex=23 |
| `txtDir` | `VB.TextBox` | Text: 0 | L=840, T=3240, W=975, H=285 | Index=8, TabIndex=22 |
| `txtDir` | `VB.TextBox` | Text: 0 | L=840, T=2880, W=975, H=285 | Index=7, TabIndex=21 |
| `txtDir` | `VB.TextBox` | Text: 0 | L=840, T=2520, W=975, H=285 | Index=6, TabIndex=20 |
| `txtDir` | `VB.TextBox` | Text: 0 | L=840, T=2160, W=975, H=285 | Index=5, TabIndex=19 |
| `txtDir` | `VB.TextBox` | Text: 0 | L=840, T=1800, W=975, H=285 | Index=4, TabIndex=18 |
| `txtDir` | `VB.TextBox` | Text: 0 | L=840, T=1440, W=975, H=285 | Index=3, TabIndex=17 |
| `txtDir` | `VB.TextBox` | Text: 0 | L=840, T=1080, W=975, H=285 | Index=2, TabIndex=16 |
| `txtDir` | `VB.TextBox` | Text: 0 | L=840, T=720, W=975, H=285 | Index=1, TabIndex=15 |
| `txtDir` | `VB.TextBox` | Text: 0 | L=840, T=360, W=975, H=285 | Index=0, TabIndex=14 |
| `Frame1` | `VB.Frame` | Caption: Ease of Access | L=120, T=4320, W=3735, H=1215 | TabIndex=0 |
| `Frame1 / btnCosts` | `VB.CommandButton` | Caption: Disable Costs | L=2280, T=720, W=1335, H=375 | TabIndex=33 |
| `Frame1 / btnRaim` | `VB.CommandButton` | Caption: Reset Aim | L=2280, T=240, W=1335, H=375 | TabIndex=2 |
| `Frame1 / btnBoff` | `VB.CommandButton` | Caption: Turn off Brownian Motion | L=120, T=240, W=2055, H=375 | TabIndex=1 |
| `Label11` | `VB.Label` | Caption: eye9 | L=240, T=3240, W=495, H=255 | TabIndex=13 |
| `Label10` | `VB.Label` | Caption: eye8 | L=240, T=2880, W=495, H=255 | TabIndex=12 |
| `Label9` | `VB.Label` | Caption: eye7 | L=240, T=2520, W=495, H=255 | TabIndex=11 |
| `Label8` | `VB.Label` | Caption: eye6 | L=240, T=2160, W=495, H=255 | TabIndex=10 |
| `Label7` | `VB.Label` | Caption: eye5 | L=240, T=1800, W=495, H=255 | TabIndex=9 |
| `Label6` | `VB.Label` | Caption: eye4 | L=240, T=1440, W=495, H=255 | TabIndex=8 |
| `Label5` | `VB.Label` | Caption: eye3 | L=240, T=1080, W=495, H=255 | TabIndex=7 |
| `Label4` | `VB.Label` | Caption: eye2 | L=240, T=720, W=495, H=255 | TabIndex=6 |
| `Label3` | `VB.Label` | Caption: eye1 | L=240, T=360, W=495, H=255 | TabIndex=5 |
| `Label2` | `VB.Label` | Caption: width | L=2040, T=120, W=735, H=255 | TabIndex=4 |
| `Label1` | `VB.Label` | Caption: direction | L=960, T=120, W=735, H=255 | TabIndex=3 |

## Getting Started: Please select your first simulation (`frmFirstTimeInfo`)

Source: [`frmFirstTimeInfo.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmFirstTimeInfo.frm)  
Class: `VB.Form`. Client size: `14115 × 11160` twips. Border style: `4`. Startup position: `2`.

| Control path | Type | Visible text or tooltip | Geometry | State and behavior |
|---|---|---|---|---|
| `Command1` | `VB.CommandButton` | Caption: &Got it! | L=6360, T=10200, W=1815, H=375 | Style=1, TabIndex=0, Appearance=0 |

## Inventory totals

- Active forms: **26**
- Declarative descendant controls and menus: **1277**
- Source baseline: public master commit `467a8a0c1564db2071cf65efe39dcfefc5ab7a0c`, whose project metadata builds `Darwin2.48.32.exe`.
