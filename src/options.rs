use strum::{Display, EnumIter, EnumProperty};

#[derive(Clone, Copy, Display, PartialEq, EnumIter, EnumProperty)]
pub enum Schema {
    #[strum(to_string = "基础版 (Base)")]
    #[strum(props(code = "base"))]
    #[strum(props(schema_id = "wanxiang"))]
    Base,
    #[strum(to_string = "专业版 (Pro)")]
    #[strum(props(code = "pro"))]
    #[strum(props(schema_id = "wanxiang_pro"))]
    Pro(Option<AuxCode>),
}

impl Schema {
    pub fn code(self) -> &'static str {
        self.get_str("code").unwrap()
    }

    pub fn schema_id(self) -> &'static str {
        self.get_str("schema_id").unwrap()
    }
}

#[derive(Clone, Copy, Display, PartialEq, EnumIter, EnumProperty)]
pub enum AuxCode {
    #[strum(to_string = "自然码")]
    #[strum(props(code = "zrm"))]
    Zrm,
    #[strum(to_string = "小鹤")]
    #[strum(props(code = "flypy"))]
    Flypy,
    #[strum(to_string = "墨奇")]
    #[strum(props(code = "moqi"))]
    Moqi,
    #[strum(to_string = "汉心")]
    #[strum(props(code = "hanxin"))]
    Hanxin,
    #[strum(to_string = "五笔前二")]
    #[strum(props(code = "wubi"))]
    Wubi,
    #[strum(to_string = "虎码首末")]
    #[strum(props(code = "tiger"))]
    Tiger,
    #[strum(to_string = "首右")]
    #[strum(props(code = "shouyou"))]
    Shouyou,
    #[strum(to_string = "首右+")]
    #[strum(props(code = "shyplus"))]
    Shyplus,
    #[strum(to_string = "万象")]
    #[strum(props(code = "wx"))]
    Wx,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, EnumIter)]
pub enum Pinyin {
    #[strum(to_string = "全拼")]
    Full,
    #[strum(to_string = "自然码")]
    Zrm,
    #[strum(to_string = "智能ABC")]
    Znabc,
    #[strum(to_string = "小鹤双拼")]
    Flypy,
    #[strum(to_string = "微软双拼")]
    Mspy,
    #[strum(to_string = "搜狗双拼")]
    Sogou,
    #[strum(to_string = "紫光双拼")]
    Ziguang,
    #[strum(to_string = "国标双拼")]
    Gbpy,
    #[strum(to_string = "拼音加加")]
    Pyjj,
    #[strum(to_string = "乱序17")]
    Lxsq,
    #[strum(to_string = "蓝天双拼")]
    Ltsp,
    #[strum(to_string = "大牛双拼")]
    Dnsp,
    #[strum(to_string = "首道双拼")]
    Sdpy,
    #[strum(to_string = "自然龙")]
    Zrlong,
    #[strum(to_string = "汉心龙")]
    Hxlong,
}

#[derive(Clone, Copy, PartialEq, Display, EnumIter)]
pub enum AuxMode {
    #[strum(to_string = "直接辅助")]
    Direct,
    #[strum(to_string = "间接辅助")]
    Indirect,
}

impl AuxCode {
    pub fn code(self) -> &'static str {
        self.get_str("code").unwrap()
    }
}
