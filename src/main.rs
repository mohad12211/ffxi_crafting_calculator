#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{fs, process::Command};

use eframe::{
    egui::{self, Button},
    emath::Align2,
};
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::<Calc>::default()),
    )
}

#[derive(Default, Serialize, Deserialize)]
struct Crystals {
    fire: String,
    earth: String,
    water: String,
    wind: String,
    ice: String,
    lightning: String,
    light: String,
    dark: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
enum Choice {
    #[default]
    NPC,
    AH,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Price {
    ah: Option<f32>,
    npc: Option<f32>,
    choice: Choice,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Item {
    name: String,
    id: String,
    stack_size: f32,
    quantity: i32,
    buy: Price,
    sell: Price,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Recipe {
    item: Item,
    output_size: f32,
    ingredients: Vec<Item>,
    crystal: Item,
    level: i32,
    produce_cost: Option<f32>,
}

impl Recipe {
    fn new(name: &str) -> Self {
        let mut ingredients: Vec<Item> = Vec::new();
        let output = grep(name, "/home/mohad/sources/AirSkyBoat/sql/synth_recipes.sql");
        for id in output.split(',').skip(13).take(8) {
            if id == "0" {
                break;
            }
            if let Some(dup) = ingredients.iter_mut().find(|x| x.id == id) {
                dup.quantity += 1;
            } else {
                ingredients.push(Item::from_id(id));
            }
        }
        let mut r = Recipe {
            item: Item::from_id(output.split(',').nth(21).unwrap()),
            output_size: output.split(',').nth(25).unwrap().parse().unwrap(),
            ingredients,
            crystal: Item::from_id(output.split(',').nth(11).unwrap()),
            level: output.split(',').nth(10).unwrap().parse().unwrap(),
            produce_cost: None,
        };
        r.calculate_produce_cost(0.0);
        r
    }

    fn calculate_produce_cost(&mut self, crystal_cost: f32) {
        let mut sum: f32 = 0.0;
        for i in &self.ingredients {
            let buy = match i.buy.choice {
                Choice::NPC => i.buy.npc,
                Choice::AH => i.buy.ah,
            };
            if let Some(buy) = buy {
                sum += i.quantity as f32 * buy;
            } else {
                self.produce_cost = None;
                return;
            }
        }
        self.produce_cost = Some(sum + crystal_cost);
    }

    fn get_value(&self) -> f32 {
        (match self.item.sell.choice {
            Choice::NPC => self.item.sell.npc.unwrap_or(0.0),
            Choice::AH => self.item.sell.ah.unwrap_or(0.0),
        } * self.output_size)
    }

    fn get_crystal_cost(&self, crystals: &Crystals) -> f32 {
        (match self.crystal.name.split('_').next().unwrap() {
            "earth" => crystals.earth.parse().unwrap_or(0.0),
            "fire" => crystals.fire.parse().unwrap_or(0.0),
            "water" => crystals.water.parse().unwrap_or(0.0),
            "wind" => crystals.wind.parse().unwrap_or(0.0),
            "ice" => crystals.ice.parse().unwrap_or(0.0),
            "lightning" => crystals.lightning.parse().unwrap_or(0.0),
            "light" => crystals.light.parse().unwrap_or(0.0),
            "dark" => crystals.dark.parse().unwrap_or(0.0),
            _ => 0.0,
        } / 12.0)
    }
}

impl Item {
    fn from_id(id: &str) -> Self {
        let output = grep(&format!("({},", id), "item_basic.sql");
        let parts = output.trim().split(',').collect::<Vec<&str>>();
        let npc_price = regex(
            &format!("^\\s+{},\\s+\\d+,.*--", id),
            "/home/mohad/sources/AirSkyBoat/scripts/zones/*",
        );
        Self {
            name: parts[3][1..parts[3].len() - 1].to_string(),
            id: parts[0][parts[0].find('(').unwrap() + 1..].to_string(),
            stack_size: parts[4].parse().unwrap(),
            buy: Price {
                ah: None,
                npc: npc_price
                    .trim()
                    .lines()
                    .map(|l| l.split(',').nth(1).unwrap().trim().parse().unwrap())
                    .min_by(f32::total_cmp),
                choice: Choice::NPC,
            },
            sell: Price {
                ah: None,
                npc: parts[8].trim().strip_suffix(");").unwrap().parse().ok(),
                choice: Choice::NPC,
            },
            quantity: 1,
        }
    }
}

fn show(str: &str) -> String {
    str.split('_')
        .map(|x| {
            let mut c = x.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

struct Calc {
    recipes: Vec<Recipe>,
    crystals: Crystals,
    load_n: usize,
    crystal_window_open: bool,
}

impl Calc {
    fn table(&mut self, ui: &mut egui::Ui) {
        let table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(eframe::emath::Align::Center))
            .column(Column::auto())
            .columns(Column::remainder().resizable(true), 5)
            .column(Column::auto());

        table
            .header(20., |mut header| {
                header.col(|ui| {
                    ui.strong("Level");
                });
                header.col(|ui| {
                    ui.strong("Recipe");
                });
                header.col(|ui| {
                    ui.strong("Crystal");
                });
                header.col(|ui| {
                    ui.strong("Cost");
                });
                header.col(|ui| {
                    ui.strong("Value");
                });
                header.col(|ui| {
                    ui.strong("Profit");
                });
                header.col(|ui| {
                    ui.strong("Details");
                });
            })
            .body(|mut body| {
                for recipe in &self.recipes {
                    body.row((18) as f32, |mut row| {
                        row.col(|ui| {
                            ui.label(recipe.level.to_string());
                        });
                        row.col(|ui| {
                            ui.label(show(&recipe.item.name));
                        });
                        row.col(|ui| {
                            ui.label(show(recipe.crystal.name.split('_').next().unwrap()));
                        });
                        row.col(|ui| {
                            ui.label(format!("{:.1}", recipe.produce_cost.unwrap_or(0.0)));
                        });
                        row.col(|ui| {
                            ui.label(recipe.get_value().to_string());
                        });
                        row.col(|ui| {
                            let ratio = recipe.item.stack_size / recipe.output_size;
                            let single = recipe.get_value() - recipe.produce_cost.unwrap_or(0.0);
                            let stack = single * ratio;
                            ui.label(format!(
                                "{:.1} (x{}) / {:.1} (x{})",
                                single, recipe.output_size, stack, recipe.item.stack_size
                            ));
                        });
                        row.col(|ui| {
                            if ui.button("Edit").clicked() {
                                println!("Clicked");
                            }
                        });
                    });
                }
            });
    }
}

impl Default for Calc {
    fn default() -> Self {
        let data = fs::read("recipes").expect("Unable to read file");
        let decoded: Vec<Recipe> = bincode::deserialize(&data[..]).unwrap();

        Self {
            recipes: decoded,
            crystals: Crystals::default(),
            load_n: 0,
            crystal_window_open: false,
        }
    }
}

impl eframe::App for Calc {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("BottomPanel").show(ctx, |ui| {
            ui.set_enabled(!self.crystal_window_open);
            ui.horizontal(|ui| {
                let button = ui.add_sized([20., 30.], Button::new("Set Crystal Price"));
                if button.clicked() {
                    self.crystal_window_open = true;
                }
                let button = ui.add_sized([20., 30.], Button::new("Save Crystal Price"));
                if button.clicked() {
                    let data: Vec<u8> = bincode::serialize(&self.crystals).unwrap();
                    fs::write("data", data).expect("Unable to write file");
                }
                let button = ui.add_sized([20., 30.], Button::new("Load Crystal Price"));
                if button.clicked() {
                    let data = fs::read("data").expect("Unable to read file");
                    let decoded: Crystals = bincode::deserialize(&data[..]).unwrap();
                    self.crystals = decoded;
                }
                let button = ui.add_sized([20., 30.], Button::new("Save Recipes"));
                ui.add(egui::Slider::new(&mut self.load_n, 0..=20));
                if button.clicked() {
                    let data: Vec<u8> = bincode::serialize(&self.recipes).unwrap();
                    fs::write("recipes", data).expect("Unable to write file");
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.crystal_window_open);
            self.table(ui);
        });
        egui::Window::new("Crystal Price")
            .open(&mut self.crystal_window_open)
            .anchor(Align2::CENTER_CENTER, [0., 0.])
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Fire Crystal: ");
                        ui.text_edit_singleline(&mut self.crystals.fire);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Earth Crystal: ");
                        ui.text_edit_singleline(&mut self.crystals.earth);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Water Crystal: ");
                        ui.text_edit_singleline(&mut self.crystals.water);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Wind Crystal: ");
                        ui.text_edit_singleline(&mut self.crystals.wind);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Ice Crystal: ");
                        ui.text_edit_singleline(&mut self.crystals.ice);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Lightning Crystal: ");
                        ui.text_edit_singleline(&mut self.crystals.lightning);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Light Crystal: ");
                        ui.text_edit_singleline(&mut self.crystals.light);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Dark Crystal: ");
                        ui.text_edit_singleline(&mut self.crystals.dark);
                    });
                    if ui.button("Recalculate").clicked() {
                        self.recipes.iter_mut().for_each(|x| {
                            x.calculate_produce_cost(x.get_crystal_cost(&self.crystals))
                        });
                    }
                })
            });
    }
}

fn read_lines<P>(filename: P) -> std::io::Result<std::io::Lines<std::io::BufReader<std::fs::File>>>
where
    P: AsRef<std::path::Path>,
{
    let file = std::fs::File::open(filename)?;
    Ok(std::io::BufRead::lines(std::io::BufReader::new(file)))
}

fn grep(pattern: &str, file: &str) -> String {
    String::from_utf8_lossy(
        &Command::new("grep")
            .arg("-i")
            .arg(pattern)
            .arg(file)
            .output()
            .expect("failed to grep")
            .stdout,
    )
    .to_string()
}

fn regex(pattern: &str, file: &str) -> String {
    String::from_utf8_lossy(
        &Command::new("bash")
            .arg("-c")
            .arg(format!("grep -Pr '{}' {}", pattern, file))
            .output()
            .expect("failed to grep")
            .stdout,
    )
    .to_string()
}
