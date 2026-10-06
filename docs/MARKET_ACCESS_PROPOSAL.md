# Voorstel: handmatige EUR/USDC-spotmarkten

Voorbereid op 5 oktober 2026. Op 6 oktober expliciet toegestaan door Willem en gericht samengevoegd met de actuele bron. Terminal en private gateways zijn bijgewerkt. De verse private accountcontrole blijft geblokkeerd door OKX-code 50110 (IP-beperking). Zie het actuele bewijs in `evidence/eur-usdc-local.json`.

## Toegestane scope

De handmatige Live-handelsroute uitbreiden van EUR naar alle EUR- en USDC-spotmarkten die het eigen OKX-account teruggeeft. Het eigen handelsbudget, risicolimieten, accountidentiteit en bestaande vrijgave blijven behouden. Nieuwe markten worden uitsluitend uitvoerbaar na eigen markt-/limietbevestiging en nieuwe Live-vrijgave. Futures, margin, andere betaalvaluta's en Live-bots vallen buiten dit voorstel. Geen order of wijziging van persoonlijke financiële instellingen door de agent.

## Geïnstalleerde wijziging

- Publieke spotcatalogus en chartfeed per bekeken markt. De publiek zichtbare lijst is geen lijst met persoonlijke handelsrechten.
- Accountlijst als bron voor toegestane spotmarkten; het formulier biedt zoeken en bewust alle EUR/USDC-markten kiezen (maximaal 2000). Alleen een eigen bevestiging slaat dit op.
- Orderbedragen, prijzen, previews en percentages gebruiken de werkelijke quotevaluta. Geen automatische EUR/USDC-conversie of transfer.
- EUR-budget-/risicotoetsen gebruiken echte gecontroleerde conversieprijzen. Een ontbrekende/verouderde waardering weigert nieuwe aankopen.
- Fees worden voor de exacte geselecteerde markt gelezen. Lokale reserveringen delen dezelfde betaal- of basismunt over meerdere paren.
- Geselecteerde markten worden ververst zonder voor iedere toegestane markt voortdurend private requests te doen. Dubieuze fills, bescherming, accountidentiteit en verliescontroles blijven blokkeren.

## Controle en grenzen

211 Rust-tests, TypeScript/productiebuild, 32 native-regressiechecks en 25 EUR/USDC-HTTP-checks slagen. HTTP-controles gebruiken uitsluitend de opgenomen wegwerpdatabase, fictieve rekeningen, publieke prijzen en lokale intents. Er draait geen private testgateway. Browserbewijs bevestigt marktkeuze zonder automatisch opslaan, USDC-prijsunits, grafiekklik met behoud van hoeveelheid en de correcte IP-blokkade.

De echte Live-accountcontrole kan pas opnieuw worden geverifieerd wanneer OKX het huidige IP voor de gebruikte API-key toelaat. Er is geen verse accountlijst of werkende Live-order geclaimd. De eerdere open gedeeltelijke-fillbescherming, herstel en volledige fase-5-acceptatie blijven open. Geen echte testorder in de gebruikersrekening geplaatst.

`docs/proposals/eur-usdc-proposal.patch` bewaart het historische voorstel. Het is niet de actuele productiebron en mag niet blind opnieuw worden toegepast. De actuele bron is met contextcontroles samengevoegd; grafiekklik, voorbeeldlimieten, publieke catalogus en persoonlijke instellingen zijn behouden. De tijdelijke prototypekopie is geen duurzaam archief. Zie `docs/ORDERS.md` en `evidence/eur-usdc-*` voor de huidige toestand.
