use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref SID: Regex = Regex::new(r#"^(?P<campaign>[^/\\]+([/\\][^/\\]+))?[/\\](?:(?P<order>[0-9]+)(?P<side>[ABCHX]?)-)?(?P<name>.+?)(?:-(?P<sideAlt>[ABCHX]?))?$"#).unwrap();
}

#[derive(Default, Copy, Clone)]
pub enum Side {
    #[default]
    A,
    B,
    C,
}

impl Side {
    pub fn parse(mode_str: &str) -> Option<Self> {
        Some(match mode_str {
            "A" | "" => Side::A,
            "B" | "H" => Side::B,
            "C" | "X" => Side::C,
            _ => return None,
        })
    }

    pub fn idx(&self) -> usize {
        match self {
            Side::A => 0,
            Side::B => 1,
            Side::C => 2,
        }
    }
}

impl ToString for Side {
    fn to_string(&self) -> String {
        match self {
            Side::A => "A".to_owned(),
            Side::B => "B".to_owned(),
            Side::C => "C".to_owned(),
        }
    }
}

pub struct SIDFields<'a> {
    pub order: usize,
    pub mode: Side,
    pub name: &'a str,
    pub campaign: &'a str,
}

impl<'a> SIDFields<'a> {
    pub fn parse(sid: &'a str) -> Result<Self, String> {
        if let Some(parsed) = SID.captures(sid) {
            let campaign = parsed.name("campaign");
            let order = parsed.name("order");
            let side = parsed.name("side").or_else(|| parsed.name("sideAlt"));
            let name = parsed.name("name");
            match (campaign, order, side, name) {
                (Some(campaign), Some(order), Some(side), Some(name))
                if !campaign.as_str().is_empty() && !order.as_str().is_empty() && !name.as_str().is_empty() => {
                    Some(Self {
                        order: order.as_str().parse().unwrap(),
                        mode: Side::parse(side.as_str()).unwrap(),
                        name: name.as_str(),
                        campaign: campaign.as_str(),
                    })
                }
                _ => None
            }
        } else {
            None
        }.ok_or_else(|| "Failed to parse SID. Must be in the format <username>/<campaign>/<order><side>-<name>, e.g. rhelmot/mymap/1B-Creekside".to_owned())
    }
}
