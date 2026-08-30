use prost::Message;
use zaino_proto::proto::service::LightdInfo;

#[test]
fn compact_tx_streamer_when_built_retains_legacy_service_name() {
    let generated = include_str!("../src/proto/service.rs");

    assert!(generated
        .contains("pub const SERVICE_NAME: &str = \"cash.z.wallet.sdk.rpc.CompactTxStreamer\";"));
}

#[test]
fn lightd_info_when_encoded_retains_existing_field_numbers() {
    let info = LightdInfo {
        version: "v".to_owned(),
        vendor: "z".to_owned(),
        taddr_support: true,
        chain_name: "main".to_owned(),
        sapling_activation_height: 1,
        consensus_branch_id: "c".to_owned(),
        block_height: 2,
        git_commit: "g".to_owned(),
        branch: "b".to_owned(),
        build_date: "d".to_owned(),
        build_user: "u".to_owned(),
        estimated_height: 3,
        zcashd_build: "x".to_owned(),
        zcashd_subversion: "s".to_owned(),
        donation_address: "a".to_owned(),
        upgrade_name: "n".to_owned(),
        upgrade_height: 4,
        lightwallet_protocol_version: "p".to_owned(),
    };

    assert_eq!(
        info.encode_to_vec(),
        b"\x0a\x01v\x12\x01z\x18\x01\x22\x04main\x28\x01\x32\x01c\x38\x02\x42\x01g\x4a\x01b\x52\x01d\x5a\x01u\x60\x03\x6a\x01x\x72\x01s\x7a\x01a\x82\x01\x01n\x88\x01\x04\x92\x01\x01p"
    );
}
