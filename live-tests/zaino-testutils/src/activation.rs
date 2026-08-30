use zaino_common::network::ActivationHeights;

/// The canonical regtest activation heights the harness launches validators with.
pub const ZEBRAD_DEFAULT_ACTIVATION_HEIGHTS: ActivationHeights = ActivationHeights {
    before_overwinter: Some(1),
    overwinter: Some(1),
    sapling: Some(1),
    blossom: Some(1),
    heartwood: Some(1),
    canopy: Some(1),
    nu5: Some(2),
    nu6: Some(2),
    nu6_1: Some(2),
    nu6_2: Some(2),
    nu6_3: Some(2),
    nu7: None,
};

/// Orchard-era-only fixture with NU6.3 disabled.
pub const ORCHARD_ONLY_ACTIVATION_HEIGHTS: ActivationHeights = ActivationHeights {
    before_overwinter: Some(1),
    overwinter: Some(1),
    sapling: Some(1),
    blossom: Some(1),
    heartwood: Some(1),
    canopy: Some(1),
    nu5: Some(2),
    nu6: Some(2),
    nu6_1: Some(2),
    nu6_2: Some(2),
    nu6_3: None,
    nu7: None,
};

/// Zebrad regtest heights with every upgrade through NU6.3 active from height 2.
pub const IRONWOOD_ONLY_ACTIVATION_HEIGHTS: ActivationHeights = ActivationHeights {
    before_overwinter: Some(1),
    overwinter: Some(1),
    sapling: Some(1),
    blossom: Some(1),
    heartwood: Some(1),
    canopy: Some(1),
    nu5: Some(2),
    nu6: Some(2),
    nu6_1: Some(2),
    nu6_2: Some(2),
    nu6_3: Some(2),
    nu7: None,
};

/// Keep in sync with [`ORCHARD_THEN_IRONWOOD_ACTIVATION_HEIGHTS`].
pub const NU6_3_TRANSITION_BOUNDARY: u32 = 6;

/// NU5 from height 2 and NU6.3 from [`NU6_3_TRANSITION_BOUNDARY`].
pub const ORCHARD_THEN_IRONWOOD_ACTIVATION_HEIGHTS: ActivationHeights = ActivationHeights {
    before_overwinter: Some(1),
    overwinter: Some(1),
    sapling: Some(1),
    blossom: Some(1),
    heartwood: Some(1),
    canopy: Some(1),
    nu5: Some(2),
    nu6: Some(2),
    nu6_1: Some(2),
    nu6_2: Some(2),
    nu6_3: Some(NU6_3_TRANSITION_BOUNDARY),
    nu7: None,
};

/// Convert zaino activation heights into zcash protocol type.
pub fn local_network_from_activation_heights(
    activation_heights: ActivationHeights,
) -> zcash_protocol::local_consensus::LocalNetwork {
    use zcash_protocol::consensus::BlockHeight;

    zcash_protocol::local_consensus::LocalNetwork {
        overwinter: activation_heights.overwinter.map(BlockHeight::from),
        sapling: activation_heights.sapling.map(BlockHeight::from),
        blossom: activation_heights.blossom.map(BlockHeight::from),
        heartwood: activation_heights.heartwood.map(BlockHeight::from),
        canopy: activation_heights.canopy.map(BlockHeight::from),
        nu5: activation_heights.nu5.map(BlockHeight::from),
        nu6: activation_heights.nu6.map(BlockHeight::from),
        nu6_1: activation_heights.nu6_1.map(BlockHeight::from),
        nu6_2: activation_heights.nu6_2.map(BlockHeight::from),
        nu6_3: activation_heights.nu6_3.map(BlockHeight::from),
    }
}

/// Convert zaino activation heights into the `zcash_local_net` type.
pub fn to_local_net_activation_heights(
    activation_heights: &ActivationHeights,
) -> zcash_local_net::protocol::ActivationHeights {
    zcash_local_net::protocol::ActivationHeights::builder()
        .set_overwinter(activation_heights.overwinter)
        .set_sapling(activation_heights.sapling)
        .set_blossom(activation_heights.blossom)
        .set_heartwood(activation_heights.heartwood)
        .set_canopy(activation_heights.canopy)
        .set_nu5(activation_heights.nu5)
        .set_nu6(activation_heights.nu6)
        .set_nu6_1(activation_heights.nu6_1)
        .set_nu6_2(activation_heights.nu6_2)
        .set_nu6_3(activation_heights.nu6_3)
        .set_nu7(activation_heights.nu7)
        .build()
}

/// Convert `zcash_local_net` activation heights into the zaino type.
pub fn from_local_net_activation_heights(
    activation_heights: &zcash_local_net::protocol::ActivationHeights,
) -> ActivationHeights {
    ActivationHeights {
        before_overwinter: activation_heights.overwinter(),
        overwinter: activation_heights.overwinter(),
        sapling: activation_heights.sapling(),
        blossom: activation_heights.blossom(),
        heartwood: activation_heights.heartwood(),
        canopy: activation_heights.canopy(),
        nu5: activation_heights.nu5(),
        nu6: activation_heights.nu6(),
        nu6_1: activation_heights.nu6_1(),
        nu6_2: activation_heights.nu6_2(),
        nu6_3: activation_heights.nu6_3(),
        nu7: activation_heights.nu7(),
    }
}
