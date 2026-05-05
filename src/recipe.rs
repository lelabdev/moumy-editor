use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeFrontmatter {
    pub title: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub manuscript: Option<String>,
    pub category: String,
    #[serde(default)]
    #[serde(rename = "categoryLabel")]
    pub category_label: Option<String>,
    #[serde(default)]
    pub excerpt: Option<String>,
    #[serde(default)]
    #[serde(rename = "prepTime")]
    pub prep_time: Option<u32>,
    #[serde(default)]
    #[serde(rename = "cookTime")]
    pub cook_time: Option<u32>,
    #[serde(default)]
    pub servings: Option<u32>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub ingredients: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub filename: String,
    pub slug: String,
    pub frontmatter: RecipeFrontmatter,
    pub steps: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecipeInput {
    pub title: String,
    pub category: String,
    #[serde(default)]
    pub category_label: Option<String>,
    #[serde(default)]
    pub excerpt: Option<String>,
    #[serde(default)]
    pub prep_time: Option<u32>,
    #[serde(default)]
    pub cook_time: Option<u32>,
    #[serde(default)]
    pub servings: Option<u32>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub ingredients: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub steps: Vec<String>,
}
