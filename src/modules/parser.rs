//use std::thread;
//use std::sync::{Arc};
use std::collections::HashSet;
use std::sync::mpsc;
//use std::borrow::Cow;

use serde_derive::{Deserialize, Serialize};
use serde_json;

#[path = "../utils/fshandler.rs"]
mod fshandler;
use fshandler::FileHandler;

#[path = "../utils/regexes.rs"]
mod regexes;
use regexes::RegexPatternManager;

#[path = "../structs/enterprise.rs"]
mod enterprise;
use enterprise::{
    EnterpriseMatrixStatistics,
    EnterpriseSubtechniquesByPlatform,
    //EnterpriseTechniquesByTactic
    EnterpriseTechnique,
    EnterpriseTechniquesByPlatform,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct EnterpriseMatrixBreakdown {
    pub tactics: HashSet<String>,
    pub platforms: HashSet<String>,
    pub datasources: Vec<String>,
    pub revoked_techniques: HashSet<(String, String)>,
    pub breakdown_techniques: EnterpriseTechniquesByPlatform,
    pub breakdown_subtechniques: EnterpriseSubtechniquesByPlatform,
    //pub breakdown_tactics:          Vec<EnterpriseTechniquesByTactic>,
    pub uniques_techniques: Vec<String>,
    pub uniques_subtechniques: Vec<String>,
    pub stats: EnterpriseMatrixStatistics,
}
impl EnterpriseMatrixBreakdown {
    pub fn new() -> Self {
        EnterpriseMatrixBreakdown {
            tactics: HashSet::new(),
            platforms: HashSet::new(),
            datasources: Vec::new(),
            revoked_techniques: HashSet::new(),
            breakdown_techniques: EnterpriseTechniquesByPlatform::new(),
            breakdown_subtechniques: EnterpriseSubtechniquesByPlatform::new(),
            //breakdown_tactics:          vec![],
            uniques_techniques: vec![],
            uniques_subtechniques: vec![],
            stats: EnterpriseMatrixStatistics::new(),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct EnterpriseMatrixParser {
    pub techniques: HashSet<String>,
    pub subtechniques: HashSet<String>,
    pub details: EnterpriseMatrixBreakdown,
}
impl EnterpriseMatrixParser {
    pub fn new() -> EnterpriseMatrixParser {
        EnterpriseMatrixParser {
            techniques: HashSet::new(),
            subtechniques: HashSet::new(),
            details: EnterpriseMatrixBreakdown::new(),
        }
    }
    pub fn baseline(&mut self, matrix_type: &str) -> Result<(), Box<dyn std::error::Error>> {
        if FileHandler::check_for_config_folder().unwrap() {
            match matrix_type {
                "enterprise" => self.baseline_enterprise()?,
                _ => (),
            }
        }
        Ok(())
    }
    fn baseline_enterprise(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _bufr = FileHandler::load_resource("matrixes", "enterprise.json");
        let _json: serde_json::Value = serde_json::from_reader(_bufr).unwrap();
        let _scanner = RegexPatternManager::load_subtechnique();
        let mut _is_subtechnique = false;
        for _t in _json["objects"].as_array().unwrap().iter() {
            let _s = _t["type"].as_str().unwrap();
            let _x = serde_json::to_string(_t).unwrap();

            if _s == "attack-pattern" && _x.contains("revoked") {
                self.extract_revoked_techniques(_t);
            } else if _s == "attack-pattern" && !_x.contains("revoked") {
                if _scanner.pattern.is_match(&_x) {
                    _is_subtechnique = true;
                    self.extract_techniques_and_tactics(_t, _is_subtechnique);
                } else {
                    _is_subtechnique = false;
                    self.extract_techniques_and_tactics(_t, _is_subtechnique);
                }
                self.extract_tactics(_t);
                if _x.contains("x_mitre_data_sources") {
                    self.extract_datasources(_t);
                }
            } else if _s == "malware" {
                self.details.stats.count_malwares += 1;
            } else if _s == "intrusion-set" {
                self.details.stats.count_adversaries += 1;
            } else if _s == "tool" {
                self.details.stats.count_tools += 1;
            }
        }
        /*
            identity
            intrusion-set
            malware
            marking-definition
            relationship
            Revoked Techniques: 0
            tool
            x-mitre-matrix
            x-mitre-tactic
        */
        //self.stats = _ems;
        //println!("\n{:?}", _ems);
        Ok(())
    }
    fn extract_revoked_techniques(
        &mut self,
        items: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _tid = items["external_references"]
            .as_array()
            .expect("Problem With External References");
        let _tid = _tid[0]["external_id"]
            .as_str()
            .expect("Problem With External ID");
        let _tname = items["name"].as_str().expect("Problem With Technique Name");
        self.details
            .revoked_techniques
            .insert((_tid.to_string(), _tname.to_string()));
        self.details.stats.count_revoked_techniques = self.details.revoked_techniques.len();
        Ok(())
    }
    fn extract_datasources(
        &mut self,
        items: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for _item in items["x_mitre_data_sources"].as_array().unwrap().iter() {
            self.details
                .datasources
                .push(_item.as_str().unwrap().to_lowercase().replace(" ", "-"));
        }
        self.details.datasources.sort();
        self.details.datasources.dedup();
        self.details.stats.count_datasources = self.details.datasources.len();
        Ok(())
    }
    fn extract_techniques_and_tactics(
        &mut self,
        items: &serde_json::Value,
        is_subtechnique: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _tid = items["external_references"]
            .as_array()
            .expect("Problem With External References");
        let _tid = _tid[0]["external_id"]
            .as_str()
            .expect("Problem With External ID");
        let _tname = items["name"].as_str().expect("Problem With Technique Name");
        let mut _platforms = String::from("");
        for _os in items["x_mitre_platforms"].as_array().unwrap().iter() {
            let _x = _os.as_str().unwrap().to_lowercase().replace(" ", "-");
            &_platforms.push_str(_x.as_str());
            &_platforms.push_str("|");
            self.details.platforms.insert(_x);
        }
        _platforms.pop();

        for _item in items["kill_chain_phases"].as_array().unwrap().iter() {
            let _tactic = &_item["phase_name"]
                .as_str()
                .expect("Problem With Killchain Phase");
            let mut _et = EnterpriseTechnique::new();
            _et.platform = _platforms.clone();
            _et.tid = _tid.to_string();
            _et.tactic = _tactic.to_string();
            _et.technique = _tname.to_string();
            let _d = items
                .as_object()
                .expect("Unable to Deserialize into String");
            // Extract Data Sources
            // Normalize the Data Source
            if _d.contains_key("x_mitre_data_sources") {
                let mut _data_sources = String::from("");
                for _ds in items["x_mitre_data_sources"]
                    .as_array()
                    .expect("Deserializing Data Sources Issue")
                {
                    _data_sources.push_str(
                        _ds.as_str()
                            .unwrap()
                            .to_lowercase()
                            .replace(" ", "-")
                            .replace("/", "-")
                            .as_str(),
                    );
                    _data_sources.push_str("|");
                }
                _data_sources.pop();
                _et.datasources = _data_sources;
                if is_subtechnique {
                    self.subtechniques.insert(_tid.to_string());
                    self.details.breakdown_subtechniques.platforms.push(_et);
                    self.details.uniques_subtechniques.push(_tid.to_string());
                } else {
                    self.techniques.insert(_tid.to_string());
                    self.details.breakdown_techniques.platforms.push(_et);
                    self.details.uniques_techniques.push(_tid.to_string());
                }
            }
        }
        // now Correlate Subtechniques
        for _record in &mut self.details.breakdown_techniques.platforms {
            for _subtechnique in &self.details.uniques_subtechniques {
                if _subtechnique.contains(&_record.tid) {
                    _record.subtechniques.push(_subtechnique.to_string());
                }
            }
            if _record.subtechniques.len() > 0usize {
                _record.has_subtechniques = true;
                _record.subtechniques.sort();
                _record.subtechniques.dedup();
            }
            _record.update();
        }
        self.details.stats.count_platforms = self.details.platforms.len();
        self.details.stats.count_active_uniq_techniques = self.techniques.len();
        self.details.stats.count_active_uniq_subtechniques = self.subtechniques.len();
        self.details.uniques_techniques.sort();
        self.details.uniques_techniques.dedup();
        self.details.uniques_subtechniques.sort();
        self.details.uniques_subtechniques.dedup();
        self.details.breakdown_subtechniques.update_count();
        self.details.breakdown_techniques.update_count();
        self.extract_stats_techniques_by_totals();
        self.extract_stats_techniques_by_platforms(false);
        self.extract_stats_techniques_by_platforms(true);
        self.extract_stats_techniques_by_killchain(false);
        self.extract_stats_techniques_by_killchain(true);
        Ok(())
    }
    fn extract_tactics(
        &mut self,
        items: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for _item in items["kill_chain_phases"].as_array().unwrap().iter() {
            self.details
                .tactics
                .insert(_item["phase_name"].as_str().unwrap().to_string());
        }
        self.details.stats.count_tactics = self.details.tactics.len();
        Ok(())
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string_pretty(&self.details).unwrap()
    }
    pub fn save_baseline(&self) {
        FileHandler::write_baseline("baseline-enterprise.json", &self.to_string());
    }
    /// # **Stats Functions**
    /// The functions in this code section baseline specific queries to offer
    /// pre-canned responses commonly needed when working with the enterprise
    /// matrix.
    ///
    /// The data processed is stored as subkeys to the `stats` key of the
    /// `EnterpriseMatrixBreakdown` struct.
    ///
    ///
    fn get_percentage(&self, total: usize, actual: usize) -> String
    {
        let _high = total as f64;
        let _low = actual as f64;
        let _percent = (_low / _high) * 100f64;
        format!("{}{}", _percent.floor().to_string(), "%")
    }
    fn extract_stats_techniques_by_totals(&mut self )
    {
        let mut _total_techniques: HashSet<String> = HashSet::new();
        let mut _total_subtechniques: HashSet<String> = HashSet::new();
        let mut _stub = String::from("");
        for _technique in self.details.breakdown_techniques.platforms.iter() {
            _stub = format!("{}:{}",_technique.tid, _technique.tactic);
            _total_techniques.insert(_stub);
        }
        for _technique in self.details.breakdown_subtechniques.platforms.iter() {
            _stub = format!("{}:{}",_technique.tid, _technique.tactic);
            _total_subtechniques.insert(_stub);
        }
        self.details.stats.count_active_total_techniques = _total_techniques.len();
        self.details.stats.count_active_total_subtechniques = _total_subtechniques.len();          
    }
    fn extract_stats_techniques_by_platforms(&mut self, _wants_subtechniques: bool)
    {
        let mut _windows_techniques:    HashSet<String> = HashSet::new();
        let mut _macos_techniques:      HashSet<String> = HashSet::new();
        let mut _linux_techniques:      HashSet<String> = HashSet::new();
        let mut _azure_ad_techniques:   HashSet<String> = HashSet::new();
        let mut _azure_techniques:      HashSet<String> = HashSet::new();
        let mut _aws_techniques:        HashSet<String> = HashSet::new();
        let mut _gcp_techniques:        HashSet<String> = HashSet::new();
        let mut _office365_techniques:  HashSet<String> = HashSet::new();
        let mut _saas_techniques:       HashSet<String> = HashSet::new();
        let mut _iterable: &Vec<EnterpriseTechnique>;
        if _wants_subtechniques {
            _iterable = &self.details.breakdown_techniques.platforms;
        } else {
            _iterable = &self.details.breakdown_subtechniques.platforms;
        }
        let mut _stub: String = String::from("");
        for _platform in self.details.platforms.iter() {
            let _os = _platform.as_str();
            for _technique in _iterable.iter() {
                if _technique.platform.contains(_os) {
                    _stub = format!("{}:{}", _technique.tid, _technique.tactic);
                    if _os == "aws" {
                        _aws_techniques.insert(_stub);
                    } else if _os == "azure-ad" {
                        _azure_ad_techniques.insert(_stub);
                    } else if _os == "azure" {
                        _azure_techniques.insert(_stub);
                    } else if _os == "gcp" {
                        _gcp_techniques.insert(_stub);
                    } else if _os == "linux" {
                        _linux_techniques.insert(_stub);
                    } else if _os == "macos" {
                        _macos_techniques.insert(_stub);
                    } else if _os == "office-365" {
                        _office365_techniques.insert(_stub);
                    } else if _os == "saas" {
                        _saas_techniques.insert(_stub);
                    } else if _os == "windows" {
                        _windows_techniques.insert(_stub);
                    }
                }
            }
        }
        if _wants_subtechniques {
            self.details.stats.count_subtechniques_aws = _aws_techniques.len();
            self.details.stats.count_subtechniques_azure = _azure_techniques.len();
            self.details.stats.count_subtechniques_azure_ad = _azure_ad_techniques.len();
            self.details.stats.count_subtechniques_gcp = _gcp_techniques.len();
            self.details.stats.count_subtechniques_linux = _linux_techniques.len();
            self.details.stats.count_subtechniques_macos = _macos_techniques.len();
            self.details.stats.count_subtechniques_office365 = _office365_techniques.len();
            self.details.stats.count_subtechniques_saas = _saas_techniques.len();
            self.details.stats.count_subtechniques_windows = _windows_techniques.len();
        } else {
            self.details.stats.count_techniques_aws = _aws_techniques.len();
            self.details.stats.count_techniques_azure = _azure_techniques.len();
            self.details.stats.count_techniques_azure_ad = _azure_ad_techniques.len();
            self.details.stats.count_techniques_gcp = _gcp_techniques.len();
            self.details.stats.count_techniques_linux = _linux_techniques.len();
            self.details.stats.count_techniques_macos = _macos_techniques.len();
            self.details.stats.count_techniques_office365 = _office365_techniques.len();
            self.details.stats.count_techniques_saas = _saas_techniques.len();
            self.details.stats.count_techniques_windows = _windows_techniques.len();            
        }
    }
    ///
    /// 
    /// 
    /// 
    fn extract_stats_techniques_by_killchain(&mut self, _wants_subtechniques: bool)
    {
        let mut _initial_access:        HashSet<String>  = HashSet::new();
        let mut _execution:             HashSet<String>  = HashSet::new();
        let mut _persistence:           HashSet<String>  = HashSet::new();
        let mut _priv_escalation:       HashSet<String>  = HashSet::new();
        let mut _defense_evasion:       HashSet<String>  = HashSet::new();
        let mut _credential_access:     HashSet<String>  = HashSet::new();
        let mut _collection:            HashSet<String>  = HashSet::new();
        let mut _discovery:             HashSet<String>  = HashSet::new();
        let mut _lateral_movement:      HashSet<String>  = HashSet::new();
        let mut _command_and_control:   HashSet<String>  = HashSet::new();
        let mut _exfiltration:          HashSet<String>  = HashSet::new();
        let mut _impact:                HashSet<String>  = HashSet::new();
        let mut _iterable: &Vec<EnterpriseTechnique>;
        if _wants_subtechniques {
            _iterable = &self.details.breakdown_techniques.platforms;
        } else {
            _iterable = &self.details.breakdown_subtechniques.platforms;
        }
        // Setup the stub
        let mut _stub: String = String::from("");
        for _tactic in self.details.tactics.iter() {
            let _kc = _tactic.as_str();
            for _technique in _iterable.iter() {
                if _technique.tactic.contains(_kc) {
                    _stub = format!("{}:{}", _technique.tid, _technique.tactic);
                    println!("STUB KILLCHAINS: {}", _stub);
                    if _kc == "initial-access" {
                        _initial_access.insert(_stub);
                    } else if _kc == "execution" {
                        _execution.insert(_stub);
                    } else if _kc == "persistence" {
                        _persistence.insert(_stub);
                    } else if _kc == "privilege-escalation" {
                        _priv_escalation.insert(_stub);
                    } else if _kc == "defense-evasion" {
                        _defense_evasion.insert(_stub);
                    } else if _kc == "credential-access" {
                        _credential_access.insert(_stub);
                    } else if _kc == "collection" {
                        _collection.insert(_stub);
                    } else if _kc == "discovery" {
                        _discovery.insert(_stub);
                    } else if _kc == "lateral-movement" {
                        _lateral_movement.insert(_stub);
                    } else if _kc == "command-and-control" {
                        _command_and_control.insert(_stub);
                    } else if _kc == "exfiltration" {
                        _exfiltration.insert(_stub);
                    } else if _kc == "impact" {
                        _impact.insert(_stub);
                    }
                }
            }
        }
        if _wants_subtechniques {
            // Subtechniques
            self.details.stats.count_subtechniques_initial_access = _initial_access.len();
            self.details.stats.count_subtechniques_execution = _execution.len();
            self.details.stats.count_subtechniques_persistence = _persistence.len();
            self.details.stats.count_subtechniques_privilege_escalation = _priv_escalation.len();
            self.details.stats.count_subtechniques_defense_evasion = _defense_evasion.len();
            self.details.stats.count_subtechniques_credential_access = _credential_access.len();
            self.details.stats.count_subtechniques_collection = _collection.len();
            self.details.stats.count_subtechniques_discovery = _discovery.len();
            self.details.stats.count_subtechniques_lateral_movement = _lateral_movement.len();
            self.details.stats.count_subtechniques_command_and_control = _command_and_control.len();
            self.details.stats.count_subtechniques_exfiltration = _exfiltration.len();
            self.details.stats.count_subtechniques_impact = _impact.len();    
        } else {
            // Techniques 
            self.details.stats.count_techniques_initial_access = _initial_access.len();
            self.details.stats.count_techniques_execution = _execution.len();
            self.details.stats.count_techniques_persistence = _persistence.len();
            self.details.stats.count_techniques_privilege_escalation = _priv_escalation.len();
            self.details.stats.count_techniques_defense_evasion = _defense_evasion.len();
            self.details.stats.count_techniques_credential_access = _credential_access.len();
            self.details.stats.count_techniques_collection = _collection.len();
            self.details.stats.count_techniques_discovery = _discovery.len();
            self.details.stats.count_techniques_lateral_movement = _lateral_movement.len();
            self.details.stats.count_techniques_command_and_control = _command_and_control.len();
            self.details.stats.count_techniques_exfiltration = _exfiltration.len();
            self.details.stats.count_techniques_impact = _impact.len(); 
        } 
    }
}
