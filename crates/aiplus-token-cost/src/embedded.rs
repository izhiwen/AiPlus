use crate::pricing::PricePerToken;

pub const EMBEDDED_SOURCE: &str = "embedded_litellm_snapshot_2026-05-19";

pub const EMBEDDED_PRICES: &[(&str, &str, PricePerToken)] = &[
    (
        "anthropic",
        "claude-opus-4-7",
        PricePerToken {
            input_usd: 0.000005,
            output_usd: 0.000025,
        },
    ),
    (
        "anthropic",
        "claude-sonnet-4-6",
        PricePerToken {
            input_usd: 0.000003,
            output_usd: 0.000015,
        },
    ),
    (
        "anthropic",
        "claude-haiku-4-5-20251001",
        PricePerToken {
            input_usd: 0.000001,
            output_usd: 0.000005,
        },
    ),
    (
        "openai",
        "gpt-5",
        PricePerToken {
            input_usd: 0.00000125,
            output_usd: 0.00001,
        },
    ),
    (
        "openai",
        "gpt-5-mini",
        PricePerToken {
            input_usd: 0.00000025,
            output_usd: 0.000002,
        },
    ),
    (
        "openai",
        "gpt-5-nano",
        PricePerToken {
            input_usd: 0.00000005,
            output_usd: 0.0000004,
        },
    ),
    (
        "openai",
        "gpt-4o",
        PricePerToken {
            input_usd: 0.0000025,
            output_usd: 0.00001,
        },
    ),
    (
        "openai",
        "gpt-4o-mini",
        PricePerToken {
            input_usd: 0.00000015,
            output_usd: 0.0000006,
        },
    ),
];
