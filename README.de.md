<p align="center"><img src="assets/banner.png" alt="Bingrove" width="100%"></p>

# Bingrove

**Gridfinity-Einsätze, die die ganze Schublade nutzen.** Bingrove plant Baseplate und Bins aus
dem Innenmaß einer Schublade, erzeugt die Dateien (STL je Teil, 3MF je Druckbett) und setzt sie
auf Drucklisten – als Plugin für [Meshgrove](https://github.com/DonDavis-vibe/MeshGrove), von
der Kommandozeile oder über eine AI wie Claude.

*English version: [README.md](README.md)*

## Das Randproblem

Ein 42-mm-Raster geht selten in einer Schublade auf. In 250 × 305 mm passt ein 5 × 7-Raster
(210 × 294 mm), und es bleiben **19,5 mm links und rechts** – ein massiver Plattenrand, in dem
nichts liegt.

Bingrove lässt das Raster, wo es ist, und lässt die Bins neben einem breiten Rand hineinragen
(„Randbins“). Die inneren Bins bleiben 100 % Gridfinity, jeder fremde Bin passt weiterhin; nur
die Bins am Rand sind etwas breiter als 42 n. Für diese Schublade steigt die Nutzfläche von 83 %
auf 97 %.

```
hinten
 #################        ~  Randzone, in die Bins hineinragen
 ~ A  A  B  B  C ~        #  fester Plattenrand (zu schmal zum Nutzen)
 ~ A  A  B  B  C ~
 ~ D  E  E  F  F ~        A: randbin_2x2_12u_L19.25.stl   D: randbin_1x5_6u_L19.25.stl
 ~ D  E  E  F  F ~        B: bin_2x2_12u.stl              E: bin_2x3_6u.stl
 ~ D  E  E  G  G ~        C: randbin_1x2_12u_L19.25.stl   …
 ~ D  I  K  M  O ~
 ~ D  H  J  L  N ~
 #################
vorne
```

Zwei Arten, den Überstand zu tragen:

- **Bis zum Boden** (Standard): Die Platte endet knapp hinter dem Raster, der Überstand des
  Randbins reicht auf einer eigenen Schürze bis zum Schubladenboden. Druckt ohne Stützen und
  gewinnt im Rand 4,75 mm Nutzhöhe.
- **Aufliegend**: Die Platte behält ihren Rand, der Überstand liegt darauf. Braucht Stützen;
  für Schubladen, in denen die Platte sich selbst festhalten muss.
- **Keine**: gar keine Randbins. Die Ränder bleiben Platte (ab 8 mm von unten ausgehöhlt), jeder
  Bin ist ein gewöhnlicher 42·n-Bin, wie im ursprünglichen Script.

Ränder unter einer Schwelle (Standard 10 mm) bleiben fester Plattenrand, damit der Einsatz
nicht rutscht.

## Was entsteht

- **Baseplate** in Teilen entlang der Rasterlinien fürs Druckbett, als offener Rahmen oder mit
  Boden und Magnetlöchern, in breiten Rändern von unten ausgehöhlt.
- **Bins** mit Stapelrand, Fächern (gleichmäßig oder auf Rasterlinien), Label-Lasche,
  Griffmulde, Magnetlöchern (Ecken oder jedes Feld) und Taschen für bestimmte Dinge (Box,
  Zylinder, Polygon).
- **Randbins** an jeder Seite, auch in Ecken.
- **Halter**: Tape-Rollen auf Ringdornen, Akkus (AA, AAA, C, D, 18650, 21700, eigenes Maß),
  1/4"-Bits, Stifte stehend oder liegend, Schraubendreher in Rinnen mit breiterer Griffmulde;
  Lochraster mit eigenem Durchmesser, Tiefe, Teilung, Anzahl und Form (rund, Sechskant,
  quadratisch) für Fräserschäfte, Bohrer und Ähnliches; **Lochreihen mit eigenen Größen**, jede
  Größe vor ihrem Loch graviert, mit Sätzen zum Laden (Nüsse 1/4", 3/8", 1/2" metrisch und
  zöllig, Inbus, Bohrer, Schraubendreher stehend) und gestuften Reihen; **Schlitze** für Karten,
  Swatches, Knopfzellen und Sägeblätter.
- **Weitere Halter**: Nüsse auf Vierkant-**Steckern** je Antrieb (1/4″, 3/8″, 1/2″; egal, wie die
  Nuss außen aussieht), **Wiegen** mit geneigten, gestuften Betten für Verlängerungen, Ratschen
  und Feilen, **V-Schlitze** für Zangen, Scheren und Schlüssel (auf einer Rampe, nach Größe
  sortiert), nach hinten **geneigte** Lochreihen für Stifte und Inbus, **Ringe** für Flaschen,
  Spraydosen und Rollen; alle mit Sätzen zum Laden und änderbaren Maßen.
- **Bin-Ausstattung**: Griffmulde und Label-Lasche an jeder Wand, offene Front,
  Fingerausschnitte in den Trennwänden, ein schräges Fenster in der Frontwand und ein
  **Aufsteckdeckel** mit Stapelrand oben, so dass Bins darauf stapeln.
- **Gravur**: eine Strichschrift (Hershey Simplex) schreibt Text in die Label-Lasche oder die
  Frontwand eines Bins und Titel auf Halter.
- **Flächen**: eine Platte für Werkbank oder Tisch, die das Raster selbst ist, Zellen bleiben
  frei, wo nichts steht; mit festem Boden (skelettiert oder voll), gesenkten
  M4-Schraublöchern, Schwalbenschwanz-Verbindern zwischen den Teilen und Magnettaschen von
  unten für Blechflächen.
- **Kalibrierteile**: eine Magnetloch-Leiter mit eingravierten Durchmessern, ein Passungstest
  mit drei Bins unterschiedlicher Luft, ein Verbinder-Paar und ein Riegel mit Nuss-Steckern, um
  einen Drucker einzustellen.
- **Ausgabe**: eine STL je Teileart mit Stückzahl, 3MF-Dateien mit den Teilen pro Druckbett
  angeordnet, der Zusammenbau als 3MF zum Anschauen und eine `*.gridfinity.json`-Projektdatei,
  die die Schublade wieder öffnet.

## Als Meshgrove-Plugin

Bingrove ist eine WebAssembly-Komponente (`wasm32-wasip2`), die Meshgrove in einer Sandbox
ausführt. Es braucht ein Meshgrove mit Plugin-System (ab 0.8.0). Die `bingrove.wasm` gibt es
bei den [Releases](https://github.com/DonDavis-vibe/Bingrove/releases), oder selbst bauen:

```sh
rustup target add wasm32-wasip2          # einmalig
scripts/build.sh                         # → target/plugin/bingrove.wasm
scripts/install-into-meshgrove.sh        # kopiert es in Meshgroves Plugin-Ordner
```

In Meshgrove: *Einstellungen → Plugins* – installieren, „AI-Tools“ und „Dateien in die
Bibliothek legen“ freigeben – dann *Plugins ▾ → Bingrove*. Das Panel hat links drei Reiter
(*Projekt*: Art, Maße, Ränder, Platte; *Belegung*: Fächer; *Ausgabe*: Drucker, Drucklisten,
Magnetprofil), in der Mitte den Plan als Raster-Editor (verschieben, Größe ändern,
zusammenlegen, teilen, auf freien Zellen neue Bins aufziehen), rechts 3D-Vorschau und
Teileliste. Ein Klick auf ein Fach öffnet darunter seine Einstellungen: Höhe, Fächer,
Label-Lasche, Griffmulde, Gravur, Halterart samt eigenen Maßen, Ausschnitte. Unter *Belegung*
gibst du an, wie viele Fächer du möchtest; Bingrove schlägt die gleichmäßigsten Mischungen von
Bin-Größen vor, die das Raster füllen („10 × 3×1“, „5 × 2×2 + 5 × 2×1“ …), ein Klick belegt die
Schublade so, und die Anzahl je Größe lässt sich danach verschieben. **Generieren + Druckliste** schreibt alles nach `Gridfinity/<Schublade>` in die
Bibliothek und legt die Drucklisten `<Schublade> – Teile` (STL mit Stückzahl) und
`<Schublade> – Druckbetten` (3MF je Bett) an; ein Klick im Verlauf macht es rückgängig.

Projektdateien (`*.gridfinity.json`) öffnen sich aus der Galerie mit „Mit Bingrove öffnen“.

### Mit einer AI

Meshgroves Chat und sein MCP-Server (`meshgrove-mcp`, für Claude Code, Claude Desktop, LM
Studio …) sehen drei Tools:

| Tool | Was es tut |
|---|---|
| `bingrove_options` | Presets, die Drucker des Nutzers, Halterkatalog, Standardwerte, Grenzen |
| `bingrove_plan` | plant eine Schublade ohne Dateien: Raster, Ränder, Teile, Textplan, Warnungen; schnell genug zum Iterieren |
| `bingrove_generate` | erzeugt die Dateien und übergibt sie Meshgrove als Paket mit den Drucklisten |

Aus *„Schublade 250 × 305 × 100 mm: Stifte, hinten zwei Kabelbins, ein Bitsatz 60 × 35 × 25 mm,
sechs 18650-Zellen“* wird ein Plan und dann ein Vorschlag, den der Nutzer in Meshgrove annimmt.
*„Eine Werkbankplatte 600 × 300 mm mit einem 1/4"-Nusssatz und einer Reihe 8-mm-Fräser“* ist
`kind: surface` mit `set: sockets_1_4_mm` und einem `holes`-Halter. Fotos laufen über den
AI-Client: Er liest die Maße aus dem Bild und übergibt Zahlen.

## Von der Kommandozeile

```sh
cargo build --release
target/release/bingrove presets
target/release/bingrove drawer 250x305x100 --name Kleinkram \
    --inhalt "Stifte:20x180x20" --inhalt "2*Kabel:bin 2x2x12" --inhalt "Bits:60x35x25" --zusammenbau
target/release/bingrove --schublade 250x305            # das Preset mit seiner festen Belegung
target/release/bingrove drawer 600x300x100 --name Bank --flaeche --plan   # eine Werkbankplatte
target/release/bingrove bin 3 1 -d 3 1 --magnete ecken  # ein einzelner Bin
target/release/bingrove tape 103 -n 2 1 -s 3            # ein Tape-Halter
```

Die Ausgabe landet in `out/<Name>/`. `--plan` gibt nur den Plan aus.

## Repository

```
crates/bingrove-core     Planung (Raster, Ränder, Teilung, Solver, Belegungsreparatur, Stückliste)
                         und mit dem Feature "geometry" die Körper und der STL-/3MF-Export
crates/bingrove-cli      der Befehl bingrove
crates/bingrove-plugin   das Meshgrove-Plugin (cdylib → bingrove.wasm)
assets/                  Icon und Banner
scripts/                 build.sh, install-into-meshgrove.sh, check.sh
```

Die Geometrie läuft auf [manifold-rust](https://crates.io/crates/manifold-rust), einem reinen
Rust-Port von Manifold – kein C++-Toolchain, derselbe Code baut für die Sandbox. Die
Gridfinity-Profile folgen
[gridfinity-rebuilt-openscad](https://github.com/kennetek/gridfinity-rebuilt-openscad); die
Schubladenlogik stammt aus einem Python-Script für zwei konkrete Schubladen, dessen 25 Teile die
Tests auf ±0,01 mm reproduzieren.

```sh
scripts/check.sh    # fmt, clippy (nativ und wasm), Tests, Plugin-Build
```

Das Plugin bezieht `meshgrove-plugin-api` direkt aus dem Meshgrove-Repository, auf einen Commit
gepinnt; der Kommentar in `crates/bingrove-plugin/Cargo.toml` zeigt, wie man es für die Arbeit an
beidem auf einen lokalen Checkout umbiegt. `.github/workflows/ci.yml` fährt `check.sh`; ein Tag
`v*` baut das Plugin und veröffentlicht es mit Prüfsumme und Changelog-Eintrag als
GitHub-Pre-Release.

## Bekannte Grenzen

- Presets beschreiben nur Schubladen; eine Werkbank beginnt bei *Art → Fläche*.
- Bins sind Rechtecke; L-förmige Bins, Wandmuster, Lite-Bins und Schiebedeckel fehlen noch.
- Lange Halter (ein Nusssatz) werden nicht fürs Druckbett geteilt; der Plan warnt, teilen musst du.
- Die Silhouette eines Bibliotheksmodells ist seine konvexe Hülle; Einbuchtungen werden aufgefüllt.
- Das Panel spricht Deutsch, die Tools und das englische README Englisch.

## Stand

Funktioniert: Planung, Randbins, Fächer-Vorschläge, Halter mit eigenen Maßen und Sätzen
(Löcher, Stecker, Reihen, Wiegen, V-Schlitze, Schlitze, Ringe, Rinnen, Tape), Bin-Ausstattung
und Deckel, Gravur, Flächen mit Schrauben, Verbindern und Magneten von unten, Ausschnitte (auch
die Silhouette eines Bibliotheksmodells), Kalibrierung, STL/3MF/Projekt-Export, CLI,
Plugin-Tools und -Panel. Kommt: Koffer mit Deckel, unregelmäßige (L-förmige) Bins, Wandmuster
und Lite-Bins, Schiebedeckel, Halbraster, ein Browser-Build, konkave Silhouetten.

## Lizenz

MIT oder Apache-2.0 nach Wahl (`LICENSE-MIT`, `LICENSE-APACHE`). Die Gridfinity-Profile folgen
der veröffentlichten Spezifikation; Gridfinity ist von Zack Freedman, Bingrove steht in keiner
Verbindung zu ihm oder gridfinity.xyz. Die Gravur nutzt die Schrift Hershey Simplex: The Hershey
Fonts were originally created by Dr. A. V. Hershey while working at the U. S. National Bureau of
Standards; the format of the font data in this distribution was originally created by James
Hurt, Cognition, Inc.
