//! Contains ser/de modules for different external types.

use std::convert::TryFrom;

use serde::{
    de::{Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};

/// Handles [`std::net::Ipv4Addr`].
pub mod ipv4 {
    use std::net::Ipv4Addr;

    use super::*;

    pub fn serialize<S>(ipv4: &Ipv4Addr, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        u32::from(*ipv4).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Ipv4Addr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ip: u32 = Deserialize::deserialize(deserializer)?;
        Ok(Ipv4Addr::from(ip))
    }
}

/// Handles [`::uuid::Uuid`].
#[cfg(feature = "uuid")]
pub mod uuid {
    use std::mem;

    use ::uuid::Uuid;

    use super::*;

    pub fn serialize<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut bytes = uuid.into_bytes();
        transform(&mut bytes);
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut bytes: [u8; 16] = Deserialize::deserialize(deserializer)?;
        transform(&mut bytes);
        Ok(Uuid::from_bytes(bytes))
    }

    /// Swaps bytes inside both 8-byte words of UUID.
    /// * Input:   0 1 2 3 4 5 6 7   8 9 10 a b c  d e f
    /// * Output:  7 6 5 4 3 2 1 0   f e  d c b a 10 9 8
    fn transform(bytes: &mut [u8; 16]) {
        let words = unsafe { mem::transmute::<&mut [u8; 16], &mut [u64; 2]>(bytes) };
        words[0] = words[0].swap_bytes();
        words[1] = words[1].swap_bytes();
    }
}

/// Handles [`::time::OffsetDateTime`].
#[cfg(feature = "time")]
pub mod time {
    use ::time::{error::ComponentRange, Date, Duration, OffsetDateTime};
    use serde::{de::Error as _, ser::Error as _};

    use super::*;

    pub mod datetime {
        use super::*;

        pub fn serialize<S>(dt: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let ts = dt.unix_timestamp();

            u32::try_from(ts)
                .map_err(|_| S::Error::custom(format!("{dt} cannot be represented as DateTime")))?
                .serialize(serializer)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
        where
            D: Deserializer<'de>,
        {
            let ts: u32 = Deserialize::deserialize(deserializer)?;
            OffsetDateTime::from_unix_timestamp(i64::from(ts)).map_err(D::Error::custom)
        }
    }

    pub mod datetime64 {
        use super::*;

        /// `DateTime64(0)`
        pub mod secs {
            use super::*;

            pub fn serialize<S>(dt: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                do_serialize(dt, 1_000_000_000, serializer)
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
            where
                D: Deserializer<'de>,
            {
                do_deserialize(deserializer, 1_000_000_000)
            }
        }

        /// `DateTime64(3)`
        pub mod millis {
            use super::*;

            pub fn serialize<S>(dt: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                do_serialize(dt, 1_000_000, serializer)
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
            where
                D: Deserializer<'de>,
            {
                do_deserialize(deserializer, 1_000_000)
            }
        }

        /// `DateTime64(6)`
        pub mod micros {
            use super::*;

            pub fn serialize<S>(dt: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                do_serialize(dt, 1_000, serializer)
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
            where
                D: Deserializer<'de>,
            {
                do_deserialize(deserializer, 1_000)
            }
        }

        /// `DateTime64(9)`
        pub mod nanos {
            use super::*;

            pub fn serialize<S>(dt: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                do_serialize(dt, 1, serializer)
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
            where
                D: Deserializer<'de>,
            {
                do_deserialize(deserializer, 1)
            }
        }

        fn do_serialize<S>(dt: &OffsetDateTime, div: i128, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let ts = dt.unix_timestamp_nanos() / div;

            i64::try_from(ts)
                .map_err(|_| S::Error::custom(format!("{dt} cannot be represented as DateTime64")))?
                .serialize(serializer)
        }

        fn do_deserialize<'de, D>(deserializer: D, mul: i128) -> Result<OffsetDateTime, D::Error>
        where
            D: Deserializer<'de>,
        {
            let ts: i64 = Deserialize::deserialize(deserializer)?;
            let ts = i128::from(ts) * mul; // cannot overflow: `mul` fits in `i64`
            OffsetDateTime::from_unix_timestamp_nanos(ts).map_err(D::Error::custom)
        }
    }

    pub mod date {
        use super::*;

        const ORIGIN: Result<Date, ComponentRange> = Date::from_ordinal_date(1970, 1);

        pub fn serialize<S>(date: &Date, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let origin = ORIGIN.unwrap();
            if *date < origin {
                let msg = format!("{date} cannot be represented as Date");
                return Err(S::Error::custom(msg));
            }

            let elapsed = *date - origin;
            let days = elapsed.whole_days();

            u16::try_from(days)
                .map_err(|_| S::Error::custom(format!("{date} cannot be represented as Date")))?
                .serialize(serializer)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
        where
            D: Deserializer<'de>,
        {
            let days: u16 = Deserialize::deserialize(deserializer)?;
            Ok(ORIGIN.unwrap() + Duration::days(i64::from(days))) // cannot overflow: always < `Date::MAX`
        }
    }
}
