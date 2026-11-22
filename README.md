# DeckGui – Nextcloud Deck für Windows
 
Ein schlankes Windows-Programm (Rust, eframe/egui), mit dem du deine **Nextcloud-Deck-Boards** bearbeiten kannst – ohne den Browser zu öffnen.
 
## Funktionen
 
- **Anmeldung** mit Server-URL, Benutzername und App-Passwort (optional gespeichert, „Angemeldet bleiben“)
- **Boards** anzeigen und neue Boards anlegen
- **Listen** (Spalten) anlegen und löschen
- **Karten** anlegen, bearbeiten und löschen
- **Drag & Drop**: Karten zwischen und innerhalb von Listen verschieben, die Änderung wird sofort an Nextcloud übertragen
- **Markdown-Editor** für die Beschreibung mit Werkzeugleiste (Fett, Kursiv, Code, Überschriften, Listen, Checkliste, Zitat, Link) und Live-Vorschau
- **Checklisten**: Checkboxen lassen sich in der Vorschau direkt anklicken
- **Fälligkeitsdatum** mit Uhrzeit (Schnellauswahl: Heute, Morgen, In 1 Woche)
- **Kartenvorschau** im Board:
  - Checkliste als `erledigt/gesamt` mit Fortschrittsbalken (ersetzt die Beschreibung)
  - Fälligkeit, farbig markiert: rot = überfällig, orange = weniger als 24 Stunden, grau = später
- Dunkles, modernes Design

## Programm bauen
 
Voraussetzung: [Rust](https://rustup.rs) unter Windows.
 
```powershell
cd deckgui
cargo build --release
```
 
Die fertige Datei liegt danach unter `target\release\deckgui.exe`. Sie kann überall hin kopiert und ohne Installation gestartet werden.
