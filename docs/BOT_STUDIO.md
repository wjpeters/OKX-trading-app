# Pine en Bot Studio · 5 oktober 2026

## Eén werkstroom

1. Schrijf een indicator of strategie in **Pine Editor**. Je concept en inputs worden lokaal in de browser hersteld wanneer je terugkomt. Kies indicatorweergave op de koersgrafiek of in een eigen paneel.
2. **Opslaan en op chart** bewaart een exacte versie, controleert de berekening en opent de terminal met het script. In de chartwerkbalk kun je andere opgeslagen versies kiezen of scripts verwijderen. Werkruimtes bewaren deze selecties.
3. **Opslaan en als botnode** bewaart en controleert het script en voegt een Pine-node aan het huidige Bot Studio-concept toe. Verbind de node met de overige stappen. Kies entry/exit-signalen of vergelijk een benoemde indicatorplot met een drempel.
4. In **Bot Studio → Bouwen** selecteer je een node voor gewone invoervelden. De templates bieden SMA, RSI en een if/else-voorbeeld. Je kunt nodes toevoegen, verwijderen, verbinden en verslepen. Elke route moet via het ordervoorstel bij de risicocontrole eindigen.
5. In **Instellingen** kies je Paper of Live, markt, orderbedrag, stop, target en budget. Verdere limieten staan ingeklapt. Opslaan bewaart een versie van de volledige grafiek én de configuratie. Dit start niets.
6. **Backtest** bewaart eerst de huidige bot en voert die precieze versie uit. Resultaten blijven op de server beschikbaar en zijn als volledig JSON-rapport te downloaden.
7. **Start & volg** toont de rekening, het botbudget en de uitvoeringsstatus. Maak hier zo nodig eerst bewust je virtuele rekening aan. **Opslaan & starten met virtueel geld** bewaart en installeert het ontwerp en start daarna de paperbot op actuele OKX-data. Een al toegepaste ongewijzigde versie kun je direct hervatten.
8. **Activiteit** toont de laatste 150 gebeurtenissen per bot, inclusief gedeelde rekeningcontroles: signalen, beslissingen, risicoweigeringen, orders en fills. De tabbladen Orders, Uitvoeringen en Posities tonen de werkelijke paperadministratie. Hetzelfde paneel staat bij **Mijn bots** en **Terminal → Bots**.

De losse Paper- en Backtestpagina’s staan niet meer in het menu. Rekeninginstellingen, posities, orders, afbouw en noodbediening zijn bereikbaar onder **Bot Studio → Start & volg → Rekeningbeheer**. Bestaande gegevens en oude rapporten zijn niet verwijderd. De oude onderzoeks-API’s blijven bestaan.

## Nodes en betekenis

- SMA-kruising, RSI-zone en Pine leveren signalen. Pine is altijd vastgezet op een exacte opgeslagen scriptversie met de daarbij opgeslagen inputs.
- **Als / anders** toetst RSI, ADX, ATR, SMA of EMA. Ja en Nee zijn afzonderlijke uitgangen; beide moeten verbonden zijn. Alleen de gekozen tak wordt uitgevoerd.
- **Alle voorwaarden** combineert de verbonden actieve takken. **Eén voorwaarde** laat elk actief signaal door; verkoop gaat voor koop. **Omgekeerd** keert een actieve booleaanse voorwaarde om, maar activeert geen niet-gekozen if/else-tak. Verkoop blijft een verkoopsignaal.
- **Koop / verkoop / wacht** maakt van een inkomend signaal een voorstel. Verkoop sluit een bestaande longpositie; het opent geen short.
- Tijdvenster werkt in UTC, inclusief het gekozen einduur. Cooldown kijkt naar de laatste kooporder. Nieuws en Jev blijven de bestaande bron-/kostencontroles gebruiken.
- Orderbedrag komt uit de botinstellingen. De verplichte risicocontrole kan een voorstel weigeren. Alle nodes blijven onder de bestaande limieten en enkele execution-gateway vallen.

## Opslaan en toepassen

Opslaan maakt een immutable versie. Een gelijktijdig opgeslagen nieuwere versie leidt tot een conflictmelding. Kopie maken krijgt een nieuw bot-id. Importeren valideert de grafiek en maakt eveneens een nieuw id. Een bestaande toegepaste bot verandert niet door alleen het ontwerp op te slaan.

Toepassen controleert Pine opnieuw, vraagt verse EUR-marktmetadata en vergrendelt de rekening voor de wijziging. Een bestaande bot moet gepauzeerd zijn, zonder open orders of posities. Zijn kapitaal, bestaande resultaatboekhouding en verliesstatus worden niet gereset. Voor een ander kapitaal maak je een nieuwe kopie. De knop bij Start & volg combineert opslaan, toepassen en de expliciete startactie. Als een stap mislukt, wordt de fout getoond; een geïnstalleerde bot blijft gepauzeerd als starten niet slaagt.

## Grenzen

- **Live is een opgeslagen ontwerpkeuze. Live starten blijft technisch geblokkeerd**, ook via het oude deploymentendpoint. De Demo/LIVE-gateway uit fase 5 is nog niet af.
- Bots en botbacktests gebruiken expliciet gesloten **1-minuutcandles**. De editor/chart kunnen andere vaste intervallen tonen; dit verandert het botinterval niet.
- Pine gebruikt het bestaande closed-bar-profiel. Strategieën geven voorstellen via numerieke `entry`/`exit`-plots. Native `strategy.*`, intrabar en `request.security` zijn niet toegevoegd. Meer dan twintig plots wordt nu expliciet geweigerd in plaats van signalen stil af te kappen.
- De botbacktest voert de nodegrafiek en de opgeslagen botlimieten uit in dezelfde risicokern. Eerste 100 bars zijn warm-up. Dataset: maximaal 5.000 beschikbare gesloten candles; daadwerkelijke periode staat in het rapport. Fills kunnen pas na de signaalcandle ontstaan. Spread/slippage zijn instelbaar, fee blijft het bestaande model van 10 bp per fill.
- Historische indicatoren/Pine beginnen op de geladen historische dataset. Doorlopende bots herberekenen de laatste 300 bars. Vooral scripts met cumulatief of langdurig geheugen kunnen daardoor afwijken. Dit is expliciet in het scherm en rapport vermeld; een backtest is geen bewijs van identieke forward-uitkomsten.
- Nieuws-/Jev-grafieken worden bij backtesten geweigerd zolang historische beslissingen ontbreken. Er worden geen filters overgeslagen en geen historische modelantwoorden verzonnen.
- Een script op de chart toont numerieke plots, met aparte signaalreeksen voor entry/exit. Niet alle Pine-tekenobjecten, stijlen of TradingView-brokersemantiek worden ondersteund.

## Verificatie

Gerichte Rust-tests voor exclusieve takken, handlevallidatie, inactieve NOT-takken, cooldown en Pine-plotselectie staan in `backend/src/graph.rs`. `scripts/studio_workflow_checks.py` maakt een disposable database op poort 4781 en controleert opslag, versieconflicten, installatie, liveblokkering, Pine en echte grafiekreplay. Hij controleert ook dat de gebruikersrekening op poort 4780 ongewijzigd is. De afzonderlijke bewijsbestanden staan onder `evidence/studio-workflow-*`.

## Live koersen met virtueel geld

Paper betekent hier forward-testen op actuele OKX-marktdata. De server verwerkt iedere nieuw gesloten 1-minuutcandle. Niet ieder signaal levert een order op: de bot wacht bijvoorbeeld als er al een positie bestaat, er geen positie te verkopen is, de data verouderd is of een risicolimiet blokkeert. Deze redenen zijn zichtbaar. Orders worden vervolgens door het papermodel gevuld, inclusief kosten en bescherming. Echt-geldhandel is een afzonderlijke, nog geblokkeerde modus.

Pauzeren stopt nieuwe strategieorders en annuleert open kooporders; bestaande posities en beschermende afbouw blijven bestaan. De server werkt verder wanneer alleen het browsertabblad sluit, maar vereist dat de lokale Docker-services en computer blijven draaien.

De activiteits-API `GET /api/paper/bots/activity?bot_id=…` leest de duurzame databasehistorie. Het paneel vernieuwt elke drie seconden. Signalen zijn geen fills; de afzonderlijke uitvoeringen zijn leidend voor daadwerkelijke virtuele transacties. De pagina’s gebruiken dezelfde beschikbare breedte en buitenmarges.

## Systeembots en handelen zonder AI

**Bot Studio → Systeembots** bevat vijf ingebouwde, onveranderlijke voorbeelden. Gebruik maakt een eigen bewerkbare kopie met een nieuw bot-id. Er wordt geen rekening, budgettoewijzing of draaiende bot aangemaakt door een voorbeeld te openen.

| Voorbeeld | Koop | Verkoop |
| --- | --- | --- |
| SMA 9 / 21 | Snelle SMA kruist omhoog door langzame SMA | Neerwaartse kruising |
| EMA 9 / 21 | Snelle EMA kruist omhoog door langzame EMA | Neerwaartse kruising |
| RSI 30 / 70 | RSI(14) onder 30 | RSI(14) boven 70 |
| MACD 12 / 26 / 9 | MACD kruist omhoog door EMA(9)-signaallijn | Neerwaartse kruising |
| Bollinger terugkeer | Slot onder onderste band, 20 candles / 2 standaardafwijkingen | Slot op of boven het gemiddelde |

De instellingen zijn voorbeeldwaarden. Alle vijf gebruiken de vaste risicokern en bevatten geen AI-, nieuws- of externe adviesnode. EMA's starten na hun SMA-warm-up; MACD gebruikt het verschil tussen de twee EMA's. Bollinger gebruikt populatiestandaardafwijking. Geen van deze indicatoren verleent zelf orderbevoegdheid.

Nieuwe bots hebben `ai_enabled: false`. Een Jev-node wordt dan ook door de backend geweigerd, zodat een ingeladen of gewijzigd document de AI-uitstand niet kan omzeilen. AI inschakelen staat afzonderlijk bij Instellingen; alleen een daadwerkelijk aangesloten Jev-node kan daarna een call aanvragen binnen de bestaande budget-/providercontroles. De verplichte risicocontrole is deterministische software en geen AI-beoordeling.

### Stop bij candle-daling

De nieuwe node berekent `(opening − slot) / opening × 100` over de laatst gesloten 1-minuutcandle. De drempel is instelbaar van 0,01% tot 50%; gelijk aan de drempel triggert ook. In de voorbeelden staat 1% met **positie sluiten en bot pauzeren**. Andere keuzes zijn alleen sluiten, of alleen pauzeren met behoud van de positie.

Deze node is een botbrede beschermingsregel: zolang hij in de geldige grafiek staat, wordt hij vóór alle normale signalen, Pine en eventuele AI beoordeeld. Ook een niet gekozen if/else-tak kan deze bescherming dus niet uitschakelen. Verwijder de node om de extra regel uit een eigen kopie te halen.

Bij een trigger worden open kooporders geannuleerd. Een sluitverzoek blijft duurzaam bewaard en blokkeert nieuwe aankopen tot de afbouw voltooid is; de bestaande uitvoerder probeert de afbouw met verse prijzen via dezelfde risicokern. De regel raakt uitsluitend de betreffende bot. Pauzeren blijft actief totdat de gebruiker de bot herstart. De activiteit toont de trigger en de latere orders/fills afzonderlijk.

Dit is een gesloten-candlecontrole, geen intrabar-dalingsalarm. De gewone stop-loss/targetbescherming blijft tussentijds op verse koersen werken. Onbeschikbare marktdata, minimumorderrestanten of een uitgeschakelde lokale server kunnen onmiddellijke uitvoering verhinderen. Backtests gebruiken dezelfde regel, met hun bestaande OHLC-uitvoermodel.
