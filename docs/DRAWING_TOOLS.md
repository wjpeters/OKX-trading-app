# Tekenen op de terminalgrafiek

Bijgewerkt op 4 oktober 2026. Alle negen tekenvormen zijn opnieuw bedienbaar via de tekenbalk links. Klik op **Tekentools** voor namen, uitleg en het benodigde aantal klikken. Beweeg de aanwijzer over een pictogram voor een korte uitleg.

## Plaatsen en aanpassen

1. Kies een tool. De onderbalk vertelt welke klik volgt; de grafiek toont een live voorbeeld.
2. Klik de punten aan. Horizontale/verticale lijnen en tekst vragen één klik. Trendlijn, Fibonacci, rechthoek en meten vragen twee klikken.
3. Een parallel kanaal vraagt een derde klik voor de breedte. Risk/reward vraagt instap, stop en koersdoel; stop en doel liggen aan verschillende kanten van de instap. Long en short worden uit de stoprichting afgeleid.
4. Na afronden keer je terug naar Selecteren. Sleep de vorm om haar te verplaatsen, of de ronde punten om haar aan te passen. Kanaalbreedte en koersdoel hebben een eigen punt.
5. De zwevende balk bij de geselecteerde tekening biedt eigenschappen, vergrendelen en verwijderen. Dubbelklikken opent eigenschappen. Kleur, tekst en prijswaarden zijn bewerkbaar. Bevestig invoer met Enter of verlaat het veld.

**Esc** of rechtsklikken annuleert het tekenen. **⌘/Ctrl Z** maakt ongedaan, **⌘/Ctrl Shift Z** of **Ctrl Y** herstelt, **Delete/Backspace** verwijdert de geselecteerde ontgrendelde tekening. Deze toetsen werken binnen de actieve grafiek; tekstvelden houden hun eigen bewerking.

De **magneet** staat standaard uit. Aan zet hij punten alleen binnen 12 schermpixels vast op de dichtstbijzijnde OHLC-prijs. **Blijf tekenen** houdt de gekozen tool actief voor meerdere vormen. **Het oog** verbergt en toont alle tekeningen zonder ze te verwijderen.

## Bewaren en weergave

- Tekeningen blijven per instrument/timeframe bewaard in dezelfde browser via de bestaande v1-opslagsleutel. Oude tekeningen blijven leesbaar. Undo/redo bewaart maximaal 50 wijzigingen per geopende grafiek; herladen bewaart de tekeningen, niet de geschiedenis.
- Er passen maximaal 100 tekeningen op een grafiek. Daarna verschijnt een melding; bestaande tekeningen verdwijnen niet automatisch.
- De tekenlaag volgt prijs en tijd, inclusief de lege ruimte naast candles. Alleen het koerspaneel is tekenbaar; volume/oscillator en assen blijven vrij.
- Koersverversingen actualiseren bestaande series en onderbreken een lopende tekening niet.
- Donkere en lichte thema's en één tot vier grafieken zijn ondersteund. Kleine multichartpanelen hebben een scrollbare indeling en tekenbalk; fullscreen geeft meer werkruimte.
- Alerts blijven een afzonderlijke expliciete knop. Een alert bewaart de lijn of eerste ankerprijs van dat moment; later tekenen wijzigt de bestaande alert niet.

## Controle op 4 oktober 2026

- `rtk proxy npm run test:drawings`: 10 geslaagde controles voor kanaalgeometrie, long/short, verplaatsen, tijdprojectie buiten candles, oude opslag en foutieve opslag.
- `rtk proxy npm run build`: TypeScript en productiebuild geslaagd. Vite meldt nog grote bundels, waaronder de bestaande Monaco-editor; dit is geen buildfout.
- Browserproef op lokale testpoort 4781 met echte publieke marktdata: alle negen vormen, tekstinvoer, trendlijnankers slepen, hele risk/reward-vorm verplaatsen, vergrendelen, verwijderen/herstellen, tekst-undo/redo, toetsenbord-undo, herhaald tekenen, Esc, verbergen/tonen en bewaren na herladen.
- Een onafgemaakte trendlijn bleef actief over ruim 50 seconden met koersverversingen. Eén/vier grafieken en beide thema's visueel gecontroleerd. Geen consolewaarschuwingen of -fouten tijdens de eindcontrole.
- De magneetknop en de implementatie van OHLC-afstand zijn gecontroleerd; geen afzonderlijke geautomatiseerde end-to-end-precisieproef voor alle zoomniveaus of touchhardware.
- Geen nieuwe serveralert, order, rekening of bot aangemaakt tijdens deze UI-proeven. Testtekeningen staan uitsluitend op de aparte testorigin.

[Voorbeeld van de nieuwe bediening](../evidence/drawing-ux-2026-10-04.jpg).

Ontwerpreferentie: [TradingView drawing tools](https://www.tradingview.com/support/solutions/43000703396-drawing-tools-available-on-tradingview/), geraadpleegd 4 oktober 2026. Gebruikt als referentie voor een herkenbare tekenbalk, magneet, herhaald tekenen, vergrendelen en ongedaan maken. Geen claim van volledige TradingView-functionaliteit.

## Lokaal beschikbaar

De geteste versie is op 4 oktober 2026 geïnstalleerd op `http://127.0.0.1:4780`. De terminalcontainer is running/healthy; de bestaande database en provider-/authconfiguratie zijn behouden. De vorige image is lokaal bewaard als `okx-tradingterminal:before-drawing-ux-20261004`. Herlaad een al geopend terminalvenster om de nieuwe interface te zien.
