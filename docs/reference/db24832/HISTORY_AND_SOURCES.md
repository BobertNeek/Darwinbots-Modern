# Darwinbots 2 history and source provenance

## 1. Executive chronology

| Date/period | Public evidence | Product significance |
|---|---|---|
| January 2002 | Carlo Comis's archived home page says “Written by Carlo Comis, Jan 2002.” | Original Darwinbots project. The page promises up to 900 organisms, shareable DNA, complex ecosystems, communication/deception, random mutation, and selection without an explicit fitness function. |
| August–December 2002 | Earliest surviving official-site captures | Public bilingual site, introduction, interface guide, message board, DNA tutorial, and downloads already exist. |
| Early 2003 | Archived introduction and interface manual | The core architecture is present: flat world, 1,000-cell memory, programmable DNA, energy cost per operation, aim-relative sensing/movement, editable species, physics, costs, mutations, autosave, and data collection. Bots were described as fixed-size squares before later circular/body-scaled rendering. |
| July 2003 | Official wiki history | Community moves from the original message board to a Proboards forum. |
| October 2003 | Official wiki history | PurpleYouko discovers tie feeding. Ties become both a competitive feeding mechanism and the basis of multibots. |
| 2003, Darwinbots 2.1 | Archived screenshot gallery | Recognizably modern desktop product: simulation arena, shots, birth ties, graphs, robot details, DNA view, console, Winamp-like gene activation, multi-tab settings, database export, and multicellular worms. |
| 2004–2005 | Wiki history and legacy readme | PurpleYouko becomes primary programmer; Numsgil joins. The forum moves from Proboards to Invision in February 2005. The collaborative wiki becomes the home page. |
| 2.2–2.3 era | Legacy “current” readme | Fixed robots, day/depth senses, proportional shot damage, shock/corpses, contest mode, restart, DNA help, 10:1 body storage, compatibility/security metadata, coordinates, target/tie memory reads. |
| 2.31–2.33 | Legacy readme | Stiff ties, RGB color editor, sexual/sperm groundwork, improved poison/venom, multibot force transfer, database snapshots, viruses and in-genome gene manipulation. |
| 2.35–2.37, 2005 | Legacy readme | Numsgil joins; broad operator expansion, mutation triggers/UI, automated leagues, reworked physics/options, seed control, zero-momentum mode, axis wrap, repopulation controls, and stealth/tray mode. |
| April 2007 | Public Git history | Repository begins with “a straight dump of 2.43's source.” |
| 2009–2010 | Public Git history and tags | Active public commits, internet-mode fixes, save fixes, project-version centralization, DB2.44 tags, then 2.45.01. |
| 2011–2012 | Public Git history and download archive | 2.45.02/2.45.03 work and branching; 2.45.01 remains known as a stable version, but is not the final Darwinbots 2 release. |
| 2014 | Download archive and source history | 2.47 adds functional leagues, internet mode, survival evolution, and automatic forking. 2.47.02 adds PlayerBot, RGB memory monitor, optimized survival, and Zerobot. 2.48 adds restriction features and a chloroplast/ecosystem overhaul. |
| 2015–2016 | Download archive and public commits | Continued mutation normalization, true-random variants, species identification, recording, internet integration, physics/shot fixes, poison changes, and survival/Zerobot stabilization. |
| 31 Oct–4 Nov 2016 | Source commit and wiki news | Project version becomes 2.48.32; public release announced 4 November. Notes: shot-array fix, fittest crash prevention, Zerobot/survival updates. |
| May 2017 | Public Git history | Last substantive project-file change fixes ActiveX relative paths. |
| August 2022 | Public Git history | Repository receives README and license files; simulation source remains 2.48.32. |
| July 2026 research date | Live wiki, forum, source repo | Wiki, forum, source, and many FTP links remain publicly described. The original Digilander site now returns HTTP 410, but Internet Archive captures preserve it. |

## 2. Original product intent

The [December 2002 archived home page](https://web.archive.org/web/20021220131805id_/http://digilander.libero.it:80/darwinbots/) is the earliest strong first-party summary found. It identifies Carlo Comis, January 2002, and an explicit design choice: evolution through mutation and environmental selection with “NO fitness function.” That principle distinguishes ordinary Darwinbots simulations from later directed Survival and Zerobot modes, which do add selection orchestration.

The [January 2003 introduction](https://web.archive.org/web/20030101221824id_/http://digilander.libero.it:80/darwinbots/introen.html) explains the model in unusual depth. Individuals act independently, carry variable-length programs, own 1,000 integers of memory, share a fixed sensor/actuator body plan, and pay energy for movement and even DNA operations. Fitness is emergent reproductive success, not stored energy alone. The modern 2.48.32 product retains all of these ideas despite extensive mechanics and UI changes.

The [February 2003 interface guide](https://web.archive.org/web/20030221115408id_/http://digilander.libero.it:80/darwinbots/interface.html) documents species setup, startup energy and position, vegetable status, field size, population limits, friction, gravity, cost/benefit controls, save/load, autosave, insertion, global mutation shocks, and data tools. It proves that the settings-heavy scientific front end is original product character rather than late accretion.

## 3. The 2.1 visual record

The official [October 2003 screenshot page](https://web.archive.org/web/20031010004035id_/http://digilander.libero.it:80/darwinbots/screenshots.htm) describes:

- Yellow Turneria Preservans hunting green Alga Minimalis.
- Colored attack particles forcing white energy particles back.
- A parent/newborn birth tie.
- Population, average mutation, and offspring graphs by species.
- Robot data, full DNA, debugger console, and real-time gene activation in a “winamp style.”
- A settings window with species, general, physics, costs, mutation, autosave, database, and internet-sharing areas.
- Programmed multicellular worms developing from one cell into precisely tied swimmers.
- External Excel/database analysis of birth cycle, mutation, DNA length, evolutionary branches, and density plots.

This record is especially valuable because it ties screenshots to intended meanings. The direct Digilander URL now returns HTTP 410, so the stable archive link above should be used for research.

## 4. Community and authorship

The official [Version History](http://wiki.darwinbots.com/w/Version_History) names Carlos/Carlo Comis as creator. It says the community created a Proboards forum in July 2003. PurpleYouko (“PY”) discovered tie feeding in October and later became primary programmer. Numsgil helped advance the simulator and repair bugs. The community moved to an Invision forum in February 2005; its first four months exceeded the old forum's total post count. The collaborative wiki later became the official home page.

The repository's [license notice](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/LICENSE.md) attributes:

- DarwinBots copyright 2003 to Carlo Comis.
- 2004–2005 modifications to Purple Youko and Numsgil.
- post-2.42 modifications in 2006–2007 to Eric Lockard.
- post-2.44.01 modifications to Darwinbots forum members.

It is a custom source/binary redistribution license with a noncommercial/nonprofit condition unless the author agrees. It is not the MIT license.

The live [Darwinbots Forum](http://forum.darwinbots.com/) remains the largest discussion archive. Forum posts are useful for bug archaeology and bot culture, but source and official release pages outrank individual recollections for 2.48.32 behavior.

## 5. Feature evolution by release family

### 5.1 2.2 and 2.3

The wiki's legacy [Readme](http://wiki.darwinbots.com/w/Readme) records 2.2 fixed-position control, day/night sensing, depth/position, proportional shot damage, shock death, and performance work around corpses. Version 2.3 added contest mode, corpse decay modes, restart, DNA Help, ten-energy-per-body storage, backward-compatible save/load, metadata protections, coordinate/tied-body/self-reference sysvars, and addressed target/tie memory reads.

### 5.2 2.31 through 2.33

2.31 added stiff-tie control and the custom RGB color window. 2.32 expanded tie reference, absolute aim, statistical contest wins, staged hereditary memory, multibot feeding/movement effects, kills, targeted poison and venom, and tie-feeding defenses. 2.33 improved internet sharing, added geometric `angle`, expanded database/snapshot analysis, and introduced viruses, virus timers, gene counts, gene deletion, and viral firing.

### 5.3 2.35 through 2.37

2.35 expanded the DNA language and mutation scripting. 2.36 automated leagues, accelerated the engine, reworked collision physics and options, and added `.thisgene` and aimed shots. 2.37 added explicit random seeds, zero translational momentum, wrap topology, repopulation controls, stealth mode, mutation statistics, and substantial save/internet fixes.

The readme calls itself current for 2.35 while containing changes through 2.37.4. It is a cumulative historical artifact, not reliable current documentation.

### 5.4 2.43 through 2.46

The public [Git repository](https://github.com/darwinbots/Darwinbots2) starts on 3 April 2007 with a 2.43 source dump. Tags cover DB2.43, several 2.44 patches, and 2.45.01/2.45.02. Commits from 2009 onward repair internet folders, velocity/refvar overflow, saving, and version metadata. The official [download archive](http://wiki.darwinbots.com/w/Main_FTP) retains 2.44.06, 2.45.01–.03, and 2.46 executables. It explicitly calls 2.45.01 stable, which likely explains why third-party sites mislabel it as the latest version.

### 5.5 2.47

The release archive credits 2.47 with fully functional leagues, working internet mode, Survival evolution, and automatic forking. Version 2.47.02 adds PlayerBot mode, RGB Memory Monitor, optimized survival evolution, and Zerobot.

### 5.6 2.48

Version 2.48 adds restriction systems and reworks the chloroplast engine for evolvable ecosystems. Later patches fix X/Y mutual gravity, chloroplast sharing, internet integration and disappearing immigrants, Eco Survival, UI and export issues, species identification, true-random builds, mutation normalization, recording, poison pricing, physics, shots, and fittest/survival/Zerobot failure cases.

The live [Main Page](http://wiki.darwinbots.com/w/Main_Page) gives these final dates:

- 26 August 2014: 2.48.
- 11 December 2015: 2.48.28.
- 30 April 2016: new 2.48.29 installer.
- 17 July 2016: 2.48.31.
- 4 November 2016: 2.48.32.

The corresponding source commit changed the VB project to `Darwin2.48.32.exe` on 31 October 2016. The wiki's release note says 2.48.32 fixed the shot array, prevented a fittest-feature crash, and updated Zerobot and survival algorithms.

## 6. Why 2.48.32 is the baseline

Several download aggregators list 2.45.01 because it had a conventional stable executable and remains widely mirrored. That is incomplete. Three independent first-party signals establish 2.48.32:

1. The official wiki news calls it released and latest.
2. The official download page supplies `Darwin2.48.32.exe` after a 2.48.29 runtime installer.
3. The public project metadata names `Darwin2.48.32.exe` with major 2, minor 48, revision 32.

2.45.01 is useful as a historical stability checkpoint, not the canonical final behavior.

## 7. Source inventory and reliability

| Source | What it establishes | Reliability and limits |
|---|---|---|
| [Public 2.48.32 source](https://github.com/darwinbots/Darwinbots2/tree/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2) | Exact executable metadata, forms, defaults, formulas, order, file behavior | Highest authority for code paths; comments may be stale and inactive code must not be mistaken for behavior |
| [Official Main Page](http://wiki.darwinbots.com/w/Main_Page) | Product premise, latest release, top-level manuals | First party; frozen after 2016 and partly vandalism-locked |
| [Official Main FTP page](http://wiki.darwinbots.com/w/Main_FTP) | Installer/update flow, release notes, tools, Linux notes | First party; some FTP/helper endpoints may now be unavailable |
| [Wiki Manual snapshot](http://wiki.darwinbots.com/) | Concepts, tutorials, individual sysvars, historical code and conventions | Rich but uneven; many pages target 2.3x or contain plans and known bugs |
| [Archived original site](https://web.archive.org/web/20021220131805id_/http://digilander.libero.it:80/darwinbots/) | Creation date, original principles, earliest features | First-party historical capture; not current behavior |
| [Archived 2.1 screenshots](https://web.archive.org/web/20031010004035id_/http://digilander.libero.it:80/darwinbots/screenshots.htm) | UI appearance and interpretation of visual states | Excellent visual/history evidence; raster images and prose describe 2.1 |
| [Live forum](http://forum.darwinbots.com/) | Community, bots, bug reports, design discussion, release attachments | Primary discussion archive; individual posts are not always final implementation |
| [Weebly guide mirror](https://darwinbots.weebly.com/guide.html) | Accessible onboarding and conceptual overview | Secondary mirror; useful corroboration, not authoritative for final formulas |
| Public Git history/tags | Code chronology from 2.43 onward | Exact commit history in repository; not a complete history before 2007 and tags stop before 2.48 |

## 8. Research capture produced for this specification

The workspace evidence set contains only public material collected for this request:

- A public source checkout pinned at commit `467a8a0c1564db2071cf65efe39dcfefc5ab7a0c`.
- A tagged 2.45.01 public worktree for historical comparison.
- A reproducible snapshot of all 1,004 namespace-zero pages from the live wiki, including revision identifiers and timestamps.
- Eight public official/wiki screenshots.
- A parsed tree of all 26 active VB6 forms.
- Generated inventories of 1,277 declarative controls and the complete compiled sysvar table.

The wiki snapshot script requests the latest revision for every main-namespace page and writes JSON Lines. It exists to make page-level evidence reproducible if the old MediaWiki instance changes or disappears.

## 9. Gaps and cautions

“All historical data online” cannot literally mean every forum message, attachment, vanished FTP binary, or unindexed personal page. The research instead captures every discoverable first-party milestone needed to explain the product, plus the complete public source behavior of the final version.

Known gaps:

- Many early binaries and attachments survive only on FTP links or forum attachments whose availability varies.
- The original site is gone from its live host and must be read through the Internet Archive.
- Source control starts at 2.43, so earlier code chronology relies on official pages and cumulative readmes.
- The wiki intermixes Darwinbots 2, proposed Darwinbots 3, plans, and obsolete source listings.
- Later public source commits after the version-number change are project-path or repository-documentation changes, not new published simulator versions.
- Live internet-mode server compatibility was not asserted. The spec describes shipped client behavior and historical service contracts.

For every contested behavior, consult the evidence precedence in the main specification: active 2.48.32 source first, then 2.48.32 forms/project data, official release material, compatible concept documentation, and finally historical discussion.
