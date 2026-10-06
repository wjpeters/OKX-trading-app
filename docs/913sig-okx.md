# 913sig voor de lokale OKX-terminal

Omgebouwd op 5 oktober 2026 uit de door Willem aangeleverde Pine v5-indicator.

## Gebruiken

Het script staat als **913sig OKX** in **Pine Editor → Opgeslagen versies**.
Kies **Type: Indicator** en **Weergave: Eigen indicatorpaneel**.
**Preview** rekent op het geselecteerde interval met gesloten OKX-candles.
**Opslaan en op chart** voegt het paneel aan de terminal toe.
De bron kan ook via **Importeer .pine** uit `pine-scripts/913sig-okx.pine` worden geladen.

## Uitvoer

- `Dalend signaal`: het labelnummer voor de dalende setup.
- `Stijgend signaal`: het labelnummer voor de stijgende setup.
- `0`: geen signaal op die candle.
- `7`, `8`, `9`: de overeenkomstige opeenvolgende setupcandle.
- `11`, `12`, `13`: bonuscounter `2`, `3`, `4`, nadat setupcounter groter dan `9` is geworden.

De vergelijking met vier candles terug en de bonusvergelijking met twee candles terug blijven gelijk. Iedere onderbreking reset de betreffende teller. Gelijkheid telt niet mee. Als alleen de bonusvoorwaarde onderbreekt terwijl de setup doorloopt, begint de bonus bij de volgende geldige candle opnieuw op 1. Daardoor kan de originele indicator binnen een lange setup opnieuw 11/12/13 tonen; deze omzetting behoudt dat gedrag.

De oorspronkelijke `na`-tellers zijn vervangen door nul-tellers. Voor de getoonde signalen is dat equivalent: een nieuwe geldige reeks begint op 1. De bonus telt uitsluitend opeenvolgende geldige candles na setup > 9. Dit is de aangeleverde eigen telregel, geen omzetting naar een andere Sequential-countdown.

## Weergaveverschil

De terminal geeft numerieke plots door en ondersteunt nog geen zichtbare Pine-labelobjecten. Ook is `syminfo.mintick` in het huidige profiel niet bruikbaar voor de oorspronkelijke labelafstand. Daarom gebruikt deze versie twee numerieke plots in een eigen paneel in plaats van gekleurde getallabels boven/onder candles. De hoogte van het signaal is het labelnummer, geen koers of rendement. De chart kiest zijn eigen plotkleuren; de oorspronkelijke kleuren per afzonderlijk label worden niet behouden.

Er zijn geen `entry`- of `exit`-plots toegevoegd. Opslaan of previewen maakt dus geen handelsvoorstellen en start geen bot.

## Verificatie

`scripts/check_913sig.py` vergelijkt alle signaalwaarden en tijdstempels in de bestaande geïsoleerde PineTS 0.11.0-worker met een referentieberekening op basis van de oorspronkelijke `na`-tellers en labelvoorwaarden. Negen scenario’s slagen, inclusief beide richtingen, gelijke koersen, herstart, bonusonderbreking zonder setupreset en 5.000 candles. Zie `evidence/913sig-verification.json`.

De werkende HTTP-preview op actuele BTC-EUR-candles en de opgeslagen scriptversie staan in `evidence/913sig-http-verification.json`. Dit is verificatie van de omzetting en het lokale runtimeprofiel. Er is geen vergelijking met een TradingView-export op identieke candles uitgevoerd. De gebruikelijke grens van circa 300 candles voor previews en herinitialisatie per run blijft gelden.
