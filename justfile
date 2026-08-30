inspect-zaino-profiles:
    cargo nextest run -p clientless --test privacy_profiles -E 'test(inspect_zaino_profiles)' --no-capture
