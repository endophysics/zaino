use zaino_proto::proto::service::LightdInfo;

use super::CapabilityError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValidatorImplementation {
    Zebra,
    Zakura,
    Development,
}

impl ValidatorImplementation {
    const fn canonical_name(self) -> &'static str {
        match self {
            Self::Zebra => "zebra",
            Self::Zakura => "zakura",
            Self::Development => "development",
        }
    }
}

#[derive(Clone, Copy)]
struct ValidatorMetadata<'a> {
    implementation: ValidatorImplementation,
    revision: &'a str,
}

enum ClassifiedMetadata<'a> {
    Absent,
    Supported(ValidatorMetadata<'a>),
    VersionOnly,
    Unsupported,
}

pub(super) fn classify(info: &LightdInfo) -> Result<(&'static str, &str), CapabilityError> {
    let build = classify_build(&info.zcashd_build);
    let subversion = classify_subversion(&info.zcashd_subversion);
    let metadata = match (build, subversion) {
        (ClassifiedMetadata::Unsupported, _)
        | (_, ClassifiedMetadata::Unsupported)
        | (ClassifiedMetadata::Absent, ClassifiedMetadata::Absent)
        | (ClassifiedMetadata::Absent, ClassifiedMetadata::VersionOnly)
        | (ClassifiedMetadata::VersionOnly, ClassifiedMetadata::Absent)
        | (ClassifiedMetadata::VersionOnly, ClassifiedMetadata::VersionOnly) => {
            return Err(CapabilityError::UnsupportedValidatorMetadata);
        }
        (ClassifiedMetadata::Supported(build), ClassifiedMetadata::Supported(subversion))
            if build.implementation != subversion.implementation =>
        {
            return Err(CapabilityError::ConflictingValidatorMetadata);
        }
        (ClassifiedMetadata::Supported(build), ClassifiedMetadata::Supported(_))
        | (ClassifiedMetadata::Supported(build), ClassifiedMetadata::Absent)
        | (ClassifiedMetadata::Supported(build), ClassifiedMetadata::VersionOnly) => build,
        (ClassifiedMetadata::Absent, ClassifiedMetadata::Supported(subversion)) => subversion,
        (ClassifiedMetadata::VersionOnly, ClassifiedMetadata::Supported(subversion)) => subversion,
    };
    if metadata.revision.chars().count() < 7 {
        return Err(CapabilityError::ShortValidatorRevision);
    }
    Ok((metadata.implementation.canonical_name(), metadata.revision))
}

fn classify_build(build: &str) -> ClassifiedMetadata<'_> {
    if build.is_empty() {
        return ClassifiedMetadata::Absent;
    }
    let implementation =
        if build.strip_prefix("Zebra ").is_some() || build.strip_prefix("zebra-").is_some() {
            ValidatorImplementation::Zebra
        } else if build.strip_prefix("zakura-").is_some() {
            ValidatorImplementation::Zakura
        } else if build.strip_prefix("development-").is_some() {
            ValidatorImplementation::Development
        } else if is_version_only_build(build) {
            return ClassifiedMetadata::VersionOnly;
        } else {
            return ClassifiedMetadata::Unsupported;
        };
    ClassifiedMetadata::Supported(ValidatorMetadata {
        implementation,
        revision: build,
    })
}

fn is_version_only_build(build: &str) -> bool {
    let Some(version) = build.strip_prefix('v') else {
        return false;
    };
    let mut components = version.split('.');
    matches!(
        (
            components.next(),
            components.next(),
            components.next(),
            components.next(),
        ),
        (Some(major), Some(minor), Some(patch), None)
            if is_version_component(major)
                && is_version_component(minor)
                && is_version_component(patch)
    )
}

fn is_version_component(component: &str) -> bool {
    !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
}

fn classify_subversion(subversion: &str) -> ClassifiedMetadata<'_> {
    if subversion.is_empty() {
        return ClassifiedMetadata::Absent;
    }
    let implementation = if subversion_matches(subversion, "/Zebra:") {
        ValidatorImplementation::Zebra
    } else if subversion_matches(subversion, "/Zakura:") {
        ValidatorImplementation::Zakura
    } else if subversion_matches(subversion, "/Development:") {
        ValidatorImplementation::Development
    } else {
        return ClassifiedMetadata::Unsupported;
    };
    ClassifiedMetadata::Supported(ValidatorMetadata {
        implementation,
        revision: subversion,
    })
}

fn subversion_matches(subversion: &str, prefix: &str) -> bool {
    subversion
        .strip_prefix(prefix)
        .and_then(|version| version.strip_suffix('/'))
        .is_some_and(|version| !version.is_empty())
}
