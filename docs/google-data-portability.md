# Google Data Portability Import Prototype

## Scope

Issue #10 evaluates whether Google Data Portability can replace or reduce cookie-based Google Maps import.

This branch adds a core prototype client and parser, not a production web OAuth flow.

## Resource Groups

The prototype targets:

- `maps.starred_places`
- `saved.collections`

Required OAuth scopes:

- `https://www.googleapis.com/auth/dataportability.maps.starred_places`
- `https://www.googleapis.com/auth/dataportability.saved.collections`

Google currently classifies `maps.starred_places` as restricted and `saved.collections` as sensitive. A production OAuth flow needs Google app verification, and restricted scopes can require security review.

## API Flow

The reusable client in `core::data_portability` supports:

1. `POST https://dataportability.googleapis.com/v1/portabilityArchive:initiate`
2. `GET https://dataportability.googleapis.com/v1/archiveJobs/{archive_job}/portabilityArchiveState`
3. Downloading signed archive URLs after the archive state is `COMPLETE`
4. Parsing exported files into the existing `GooglePlace` model

The client expects an already-authorized OAuth access token with the Data Portability scopes. This avoids reintroducing the existing basic Google OAuth sign-in flow, which did not provide Maps saved-list access.

## Parsed Export Formats

- `maps.starred_places`: GeoJSON, mapped to title, Google Maps URL, address, latitude, longitude, and source tag.
- `saved.collections`: CSV, mapped to title, URL, notes, comments, and collection/list tag when present.

Existing Google Maps URI enrichment can still fill missing coordinates and place details after parsing.

## Current Finding

Google Data Portability is a plausible official import path for Starred Places and saved collections, but it cannot be treated as a drop-in replacement until the OAuth app is approved for the sensitive/restricted Data Portability scopes and tested against a real account archive.

Cookie import remains necessary as the working path for custom Google Maps saved lists until this verification and archive-shape testing is complete.

## References

- Google Data Portability REST API: https://developers.google.com/data-portability/reference/rest
- Initiate archive method: https://developers.google.com/data-portability/reference/rest/v1/portabilityArchive/initiate
- Archive state method: https://developers.google.com/data-portability/reference/rest/v1/archiveJobs/getPortabilityArchiveState
- OAuth scopes: https://developers.google.com/data-portability/user-guide/scopes
- Maps starred places schema: https://developers.google.com/data-portability/schema-reference/local_actions
- Saved collections schema: https://developers.google.com/data-portability/schema-reference/save
