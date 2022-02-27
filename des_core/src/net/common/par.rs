use std::str::FromStr;

use crate::ModulePath;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Parameter {
    path_and_name: String,
    name_start: usize,
    value: String,
}

impl Parameter {
    pub fn path(&self) -> &str {
        if self.name_start == 0 {
            &self.path_and_name[..0]
        } else {
            &self.path_and_name[..self.name_start - 1]
        }
    }

    pub fn key(&self) -> &str {
        &self.path_and_name[self.name_start..]
    }

    pub fn new(key: &str, value: &str) -> Self {
        match key.chars().rev().enumerate().find(|(_, c)| *c == '.') {
            Some((idx, _)) => Self {
                path_and_name: key.to_string(),
                name_start: idx + 1,
                value: value.to_string(),
            },
            None => Self {
                path_and_name: key.to_string(),
                name_start: 0,
                value: value.to_string(),
            },
        }
    }

    pub fn parse<F: FromStr>(&self) -> Result<F, <F as FromStr>::Err> {
        self.value.parse()
    }
}

pub struct Parameters {
    pars: Vec<Parameter>,
}

impl Parameters {
    pub fn empty() -> Self {
        Self { pars: Vec::new() }
    }

    pub fn add(&mut self, string: String) {
        for line in string.lines() {
            let splits: Vec<&str> = line.split("=").collect();
            if splits.len() == 2 {
                self.pars.push(Parameter::new(splits[0], splits[1]))
            }
        }
    }

    pub fn for_module(&self, module: &ModulePath) -> Vec<Parameter> {
        self.pars
            .iter()
            .filter(|p| p.path() == module.module_path())
            .cloned()
            .collect()
    }

    pub fn get(&self, module_path: &ModulePath, name: &str) -> Option<&Parameter> {
        let search_term = format!("{}.{}", module_path, name);
        self.pars.iter().find(|p| p.path_and_name == search_term)
    }
}
