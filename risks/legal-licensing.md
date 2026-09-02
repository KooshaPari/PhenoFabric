# Legal, Licensing, and Distribution Risk

This is an engineering risk checklist, not legal advice.

## Required inventories

- source and binary licenses for every adapter/library;
- codec patent/licensing obligations by distribution and hardware path;
- driver SDK and signing terms;
- vendor service terms governing automation, embedding and redistribution;
- trademarks/product names in UI and documentation;
- privacy/recording consent for screen, audio, microphone and input logs;
- cryptography/export and hosted-relay jurisdiction where applicable;
- game anti-cheat, DRM and application terms for capture/injection/VM use.

## Architecture mitigations

- prefer process-boundary adapters when license compatibility is uncertain;
- do not copy code merely because a project is public;
- ship an SBOM and machine-readable notices;
- keep optional commercial integrations separately enabled/configured;
- do not bypass secure surfaces, anti-cheat or DRM;
- make recording/capture indicators and retention explicit;
- retain source attribution and upstream modification history;
- require legal review before distributing codecs/drivers/vendor integrations at scale.

## AI-generated implementation

Every generated contribution still requires provenance, license scanning, review and tests. “Generated” is not evidence that code is free of copied material or that the surrounding product has no contractual obligations.
