# Deployment telemetry

This module defines a minimal schema for emitting privacy-first telemetry events tied to a zone deployment.

The intention is *measurement without surveillance*: telemetry should avoid personal data, prefer aggregation,
and record the lockfile digest so results remain attributable to a precise stack state.
