from __future__ import annotations


from tools import msez


def test_constrained_netting_multi_corridor_multi_currency_deterministic():
    """Hard-route netting: multi-corridor + multi-currency + constraints + deterministic tie-breaks."""

    obligations = [
        # USD obligations (2 independent edges that will be netted into a different pairing)
        {
            "debtor_party_id": "A",
            "creditor_party_id": "B",
            "amount": {"currency": "USD", "value": "100"},
        },
        {
            "debtor_party_id": "D",
            "creditor_party_id": "C",
            "amount": {"currency": "USD", "value": "100"},
        },
        # EUR obligation (kept as-is)
        {
            "debtor_party_id": "E",
            "creditor_party_id": "F",
            "amount": {"currency": "EUR", "value": "50"},
        },
    ]

    settlement_corridors = [
        {"corridor_id": "corridor.alpha", "currency": "USD", "priority": 1},
        {"corridor_id": "corridor.beta", "currency": "USD", "priority": 1},
        {"corridor_id": "corridor.eur", "currency": "EUR", "priority": 1},
    ]

    constraints = {
        # Force the algorithm to avoid the original A->B leg.
        "blocked_pairs": [
            {"from_party_id": "A", "to_party_id": "B", "currency": "USD"},
        ],
        # Force corridor selection to use corridor.beta whenever C participates.
        "party_corridor_allowlist": {
            "C": ["corridor.beta"],
        },
    }

    netting, legs = msez._compute_netting_and_legs(
        obligations,
        settlement_corridors=settlement_corridors,
        constraints=constraints,
    )

    # Netting is a flat list, sorted by (currency, party_id)
    assert netting == [
        {"party_id": "E", "amount": {"currency": "EUR", "value": "-50"}},
        {"party_id": "F", "amount": {"currency": "EUR", "value": "50"}},
        {"party_id": "A", "amount": {"currency": "USD", "value": "-100"}},
        {"party_id": "B", "amount": {"currency": "USD", "value": "100"}},
        {"party_id": "C", "amount": {"currency": "USD", "value": "100"}},
        {"party_id": "D", "amount": {"currency": "USD", "value": "-100"}},
    ]

    # Legs are sorted deterministically; EUR comes before USD.
    assert legs == [
        {
            "leg_id": "EUR:000000",
            "from_party_id": "E",
            "to_party_id": "F",
            "amount": {"currency": "EUR", "value": "50"},
            "settlement_corridor_id": "corridor.eur",
        },
        {
            "leg_id": "USD:000000",
            "from_party_id": "A",
            "to_party_id": "C",
            "amount": {"currency": "USD", "value": "100"},
            "settlement_corridor_id": "corridor.beta",
        },
        {
            "leg_id": "USD:000001",
            "from_party_id": "D",
            "to_party_id": "B",
            "amount": {"currency": "USD", "value": "100"},
            "settlement_corridor_id": "corridor.alpha",
        },
    ]


def test_constrained_netting_infeasible_raises():
    """When constraints make settlement impossible, the algorithm must fail loudly."""

    obligations = [
        {"debtor_party_id": "A", "creditor_party_id": "B", "amount": {"currency": "USD", "value": "10"}},
    ]
    settlement_corridors = [{"corridor_id": "corridor.usd", "currency": "USD", "priority": 1}]
    constraints = {
        "blocked_pairs": [{"from_party_id": "A", "to_party_id": "B", "currency": "USD"}],
    }

    try:
        msez._compute_netting_and_legs(
            obligations,
            settlement_corridors=settlement_corridors,
            constraints=constraints,
        )
        assert False, "expected ValueError"
    except ValueError as ex:
        assert "unable to produce" in str(ex)
