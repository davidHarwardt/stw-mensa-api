
use std::str::FromStr;

use chrono::NaiveDate;
use scraper::{Html, Selector, ElementRef};
use serde::{Serialize, Deserialize};

fn sel(v: &str) -> Selector { Selector::parse(v).unwrap() }

fn text_content<'a>(t: &ElementRef<'a>) -> String {
    t.text().map(|v| v.trim()).collect::<Vec<_>>().join(" ").trim().to_owned()
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color { Green, Orange, Red }

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum MealTag {
    Vegetarian,
    Vegan,
    Fairtrade,
    ClimateFood,
    SustainableFarming,
    SustainableFishing,
    Frozen,
    Co2(Color),
    WaterUsage(Color),
    Quality(Color),
}

impl MealTag {
    pub fn from_name(v: &str) -> Option<Self> {
        Some(match v.trim() {
            "gruen" => Self::Quality(Color::Green),
            "gelb" => Self::Quality(Color::Orange),
            "rot" => Self::Quality(Color::Red),
            "vegetarisch" => Self::Vegetarian,
            "vegan" => Self::Vegan,
            "bio" => Self::SustainableFarming,
            "klima" => Self::ClimateFood,
            "msc" => Self::SustainableFishing,

            "CO2_bewertung_A" => Self::Co2(Color::Green),
            "CO2_bewertung_B" => Self::Co2(Color::Orange),
            "CO2_bewertung_C" => Self::Co2(Color::Red),

            "H2O_bewertung_A" => Self::WaterUsage(Color::Green),
            "H2O_bewertung_B" => Self::WaterUsage(Color::Orange),
            "H2O_bewertung_C" => Self::WaterUsage(Color::Red),
            _ => None?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meal {
    pub name: String,
    pub price: MealPrice,
    pub tags: Vec<MealTag>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MealPrice {
    pub student: u64,
    pub medium: u64,
    pub expensive: u64,
}

impl FromStr for MealPrice {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, prices) = s.trim().split_once(" ").ok_or(())?;
        let prices = prices.replace(",", ".").split("/")
            .map(|v| v.parse::<f64>().map_err(|_| ()))
            .map(|v| v.map(|v| (v * 100.0) as u64))
        .collect::<Result<Vec<_>, ()>>()?;
        if prices.len() < 3 {
            tracing::warn!("price has invalid format");
            Err(())?
        }
        Ok(MealPrice {
            student: prices[0],
            medium: prices[1],
            expensive: prices[2],
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MealGroup {
    pub name: String,
    pub meals: Vec<Meal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MensaMenu {
    pub date: NaiveDate,
    pub groups: Vec<MealGroup>,
}

#[derive(thiserror::Error, Debug)]
pub enum MenuError {
    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error("could not find category name")]
    CategoryNameNotFound,
    #[error("could not find meal name")]
    MealNameNotFound,
    #[error("could not find meal price")]
    MealPriceNotFound,
}

impl MensaMenu {
    pub async fn load(
        client: &reqwest::Client,
        mensa_id: impl Into<String>,
        date: NaiveDate,
    ) -> Result<Self, MenuError> {
        let url = "https://www.stw.berlin/xhr/speiseplan-wochentag.html";

        let res = client.post(url)
            .form(&[
                ("date", date.format("%Y-%m-%d").to_string()),
                ("resources_id", mensa_id.into()),
            ])
            .send()
            .await?
            .text()
        .await?;

        let doc = Html::parse_document(&res);

        let meal_list_sel = sel(".splGroupWrapper");
        let group_name_sel = sel(".splGroup");
        let meal_sel = sel(".splMeal");

        let meal_name_sel = sel("span.bold");
        let price_sel = sel("div.text-right");
        let tag_sel = sel("span[role = 'tooltip']");

        let mut res = Self {
            date,
            groups: Vec::new(),
        };
        for group in doc.select(&meal_list_sel) {
            let category_name = group.select(&group_name_sel)
                .next()
                .ok_or(MenuError::CategoryNameNotFound)?
            .inner_html();
            let mut category = MealGroup {
                name: category_name,
                meals: Vec::new(),
            };

            for meal in group.select(&meal_sel) {
                let name_field = meal.select(&meal_name_sel)
                    .next()
                .ok_or(MenuError::MealNameNotFound)?;
                let name = text_content(&name_field);

                let tags = meal.select(&tag_sel)
                    .filter_map(|v| MealTag::from_name(&v.inner_html()))
                .collect();

                let price_field = meal.select(&price_sel)
                    .next()
                .ok_or(MenuError::MealPriceNotFound)?;
                let price = text_content(&price_field)
                    .parse()
                .map_err(|_| MenuError::MealPriceNotFound)?;
                category.meals.push(Meal {
                    name, price, tags,
                });
            }

            res.groups.push(category);
        }
        Ok(res)
    }
}

