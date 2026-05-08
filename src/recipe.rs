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
    pub excerpt: Option<String>,
    #[serde(default)]
    #[serde(rename = "prepTime")]
    pub prep_time: Option<String>,
    #[serde(default)]
    #[serde(rename = "cookTime")]
    pub cook_time: Option<String>,
    #[serde(default)]
    pub servings: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub ingredients: Vec<String>,
    /// Title for the second ingredient list (e.g. "Pour la pâte")
    #[serde(default, rename = "ingredients2Title")]
    pub ingredients2_title: Option<String>,
    /// Optional second ingredient list (items only, no title)
    #[serde(default, rename = "ingredients2")]
    pub ingredients2: Option<Vec<String>>,
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
    pub slug: Option<String>,
    pub category: String,
    #[serde(default)]
    pub excerpt: Option<String>,
    #[serde(default)]
    pub prep_time: Option<String>,
    #[serde(default)]
    pub cook_time: Option<String>,
    #[serde(default)]
    pub servings: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub ingredients: Vec<String>,
    /// Title for the second ingredient list (e.g. "Pour la pâte")
    #[serde(default)]
    pub ingredients2_title: Option<String>,
    /// Items for the second ingredient list (title NOT included)
    #[serde(default)]
    pub ingredients2: Option<Vec<String>>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub steps: Vec<String>,
}
