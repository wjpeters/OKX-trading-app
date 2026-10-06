# Systeem: API-sleutels en Jev · 5 oktober 2026

Open **Systeem → Aansluitingen & AI** in de lokale terminal. **Status & bescherming** bevat de bestaande verbindingen, latency, opslag, toegang en vrijgavevoorwaarden.

## Gebruik

1. OKX: kies **Demo** of **Live** en vul API-key, API secret en passphrase van dezelfde key in. De twee sleutelsets blijven gescheiden. Opslaan start geen onboarding of handel. Demo-onboarding lees je uit met de bestaande afzonderlijke helper; LIVE-uitvoering blijft niet beschikbaar.
2. Jev: vul een TypeSafe-key in bij **Jev AI direct**, of een OpenRouter-key bij **Jev AI via OpenRouter**. Beide mogen opgeslagen zijn.
3. Kies onder **Welke Jev moet worden gebruikt?** de gewenste provider en druk op **Provider opslaan**. Deze keuze geldt voor handmatige voorstellen en Jev-nodes. Er is geen automatische fallback.
4. Vul je eigen totale AI-budget in USD in, bevestig de kosten en kies **Activeer Jev**. Opslaan van een sleutel of provider activeert geen betaalde aanvragen. **Stop Jev** schakelt voorstellen uit. Bij een providerwissel of AI-sleutelwijziging via Systeem moet je het budget opnieuw activeren.

Sleutelvelden blijven leeg na opslag en bij herladen. Leeg laten overschrijft niets; voor vervangen vul je een nieuwe key in, voor OKX steeds de complete set. Verwijderen vraagt een bevestiging. De status betekent **lokaal opgeslagen**, geen geverifieerde verbinding. De app test bij opslaan geen provider, voert geen betaalde proefaanvraag uit en plaatst geen order.

## Providers en kosten

| Keuze | Model | Route | Kosten |
| --- | --- | --- | --- |
| Jev direct | `jev-1.13.0` | TypeSafe System One | Berekend uit werkelijk ontvangen inputtokens tegen $0,042 per miljoen; output gratis volgens het vastgelegde tarief van 2026-10-05 |
| Jev via OpenRouter | `typesafe/jev-1.13` | OpenRouter Decisions | USD-kosten uit de providerrespons |

TypeSafe geeft in het gedocumenteerde schema geen factuurbedrag of request-id terug. Daarom markeert de terminal het bedrag als berekend en het id als lokaal. De providerfactuur blijft leidend. Tariefwijzigingen vereisen een update; geen kosten als werkelijk gefactureerd presenteren. Bron: [TypeSafe API](https://docs.typesafe.ai/api), [model en tarief](https://docs.typesafe.ai/models), [OpenRouter Decisions](https://openrouter.ai/blog/tutorials/how-to-use-jev/).

Modelversie, typed choice, confidence, probabilities en gebruik blijven gevalideerd. De bestaande kostenreserve, deadline, circuit-breaker en vaste risicokern blijven gelden. Kosten en reserveringen worden niet gereset bij wisselen. Sleutel-/providerwijzigingen worden geweigerd tijdens een lopende Jev-aanvraag.

## Lokale opslag en onderhoud

- Versleutelde envelopes: `.local/provider-settings/` (map 700, bestanden 600), alleen lokaal buiten iCloud. Masterkey: `.local/secrets.key` (600), afzonderlijk aan de container aangeboden.
- AES-256-GCM met willekeurige nonce en gebonden sleuteltype. Atomair vervangen; verwijderen schrijft een leeg versleuteld record zodat een oude key niet terugkomt.
- Alleen presence-flags verlaten de backend. Geen keywaarden in API-antwoorden, logs, browseropslag, Git of images. Invoer gaat alleen naar de lokale server en blijft kortstondig in het invoerveld tot opslag of verlaten van de pagina.
- Bestaande versleutelde OpenRouter-/Demo-envelopes worden bij starten eenmalig lokaal overgenomen. Oude bronbestanden worden behouden maar niet meer gebruikt door de terminal of Demo-helper.
- De oude CLI-setup werkt alleen bij een gestopte terminal; gebruik bij een actieve app Systeem. De Demo-helper krijgt uitsluitend de Demo-key en kan geen Live-key lezen.
- Bestaande database-/Parquetbackups bevatten deze credentialmap en masterkey niet. Bewaar ze alleen via een afzonderlijke lokale beveiligde backup. Zonder masterkey zijn de envelopes niet te herstellen.

## Verificatie

114 Rust-tests (104 terminal + 10 onboarding), TypeScript/productiebuild en 40 geïsoleerde integratiecontroles slagen. Met uitsluitend synthetische keys gecontroleerd: encryptie en bestandsrechten, geen echo, onvolledige invoer, CSRF, afzonderlijke omgevingen, wijziging/verwijdering, herstartpersistentie, tamperdetectie, providerwissels en behoud van niet-nulle kosten/reserveringen.

Browserproef: instellingen bereikbaar, key opslaan en invoer wissen, provider kiezen, herladen, Demo/Live-velden, verwijderbevestiging annuleren, beide Systeem-tabs en lichte/donkere weergave. Geen gebruikersrekening, bots of betaalde providerbeslissingen aangemaakt. Echte provideracceptatie en OKX Demo-accountacceptatie blijven open zonder nieuwe gebruikerskeys en expliciete budgetten.

Bewijs: `evidence/system-settings-rust-tests.txt`, `evidence/system-settings-build.txt`, `evidence/system-settings-integration.json`.
