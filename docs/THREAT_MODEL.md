# Threat model · lokale paper-/onderzoeksrelease

| Grens | Maatregel | Restrisico/open werk |
| --- | --- | --- |
| Browser/AI/script probeert orderrechten te krijgen | Vaste centrale risk/OMS, hercontrole, atomaire reservering; paperbinary importeert private exchange-module niet | De OS-gebruiker/Dockerbeheerder is vertrouwd; geen sandbox tegen die beheerder |
| Cross-site/DNS rebinding | Exacte Host/Origin, geen CORS, CSRF-header, CSP, body16KiB, 4 gelijktijdige mutaties/60 per minuut | Niet publiek of via tunnel blootstellen; onafhankelijke review ontbreekt |
| Lokale toegang | Optionele PBKDF2-SHA256 600.000 iteraties, HttpOnly/SameSiteStrict-cookie, gehashte 8h-sessies, WS-hercontrole | Standaard alleen localhost; geen MFA/TLS/VPN in deze release |
| Pine-code | Echte parser, fail-closed profiel, nieuwe processen, Node permissions, netwerkloze non-root container, CPU/RAM/deadline/outputlimieten | Geen bewijs dat alle upstreamsyntax/semantiek veilig en correct is; kernel/Docker/Node blijven vertrouwd |
| Jev/nieuwsinjectie | Vaste hosts, escaped/schoongemaakte tekst, URLallowlist, geen tools/orderrechten, schemavalidatie, snapshot/versie/candle/TTL-hercontrole | Echte Jev-key/call en adversarial provideracceptatie nog niet gedaan |
| Kostenrunaway | Eén actieve call, DB-lock, vooraf kostenreserve, account/bot-id-caps, breaker | Onbekende kosten houden reserve; quota/prijzen kunnen wijzigen |
| Datagat/stale/klok | books-sequence, reconnect/resync, candle-gaps, freshmetadata/price/klokgate | Native exchangebescherming ontbreekt; lokale stop werkt niet tijdens hostuitval |
| DB/crash/dubbele levering | Durable intent/reserve, rijlocks, client-id/filldedup, pending/unknown, persistente HWM/dagstart | Exchange-timeout/fencing is nog niet end-to-end geïmplementeerd |
| Storage | Waarschuwing >=90%, instappauze >=95% of onbekende meting; versleutelde backup/restoretest | Drempeltest is geen fysieke disk-fullproef; backups nu handmatig en <=256MiB |
| Credentials | AES256-GCM envelope met purpose-AAD; aparte lokale sleutel buiten DB; verborgen menselijke setup; .local uitgesloten van Git/images | DB-password zelf lokaal600; Compose mount is geen encryptie. Zelfde hostgebruiker kan key en ciphertext lezen |
| Supply chain | Lockfiles/digests, cargo/npm audits, netwerkloze Trivy-imagescan, kleinere runtimes | Geen garantie tegen onbekende kwetsbaarheden; scanresultaten momentopname |

App- en Pinecontainer read-only/non-root, cap_drop ALL, no-new-privileges, begrensde tmpfs/pids/CPU/RAM. Database alleen intern netwerk. Geen Docker-socket. Voor de scanner wordt alleen de openbare kwetsbaarheidsdatabase online opgehaald zonder projectmount; inspectie van projectimages draait `--network none`, zonder socket of upload.

Docker Scout is niet uitgevoerd: automatische goedkeuringsreview weigerde de mogelijke verzending van image-/SBOM-metadata naar Docker. Trivy voert de vervangende scan volledig lokaal uit. Zie `evidence/*-image.json`; eerdere bevindingen blijven onder `*-before-hardening.json` behouden.

LIVE is geblokkeerd. Er is geen bevestigde echte rekening of geactiveerde gebruikersbot. Geen private keys ingevoerd, geen betaalde modelcall en geen exchangeorder uitgevoerd tijdens ontwikkeling.
