use std::borrow::Cow;

use colored::Colorize;
use regex::{Regex, RegexBuilder, Replacer};
use serde_json::Value;
use tracing::warn;

use crate::{
    api::AppState,
    completion::{Message, RetryStrategy},
    utils::REPLACEMENT,
};

impl AppState {
    pub fn handle_messages(
        &self,
        messages: &[Message],
        strategy: RetryStrategy,
    ) -> (String, Vec<String>) {
        let re_scenario =
            RegexBuilder::new(r"^\[Circumstances and context of the dialogue: ([\s\S]+?)\.?\]$")
                .case_insensitive(true)
                .build()
                .unwrap();
        let re_personality = RegexBuilder::new(r"^\[([\s\S]+?)'s personality: ([\s\S]+?)\]$")
            .case_insensitive(true)
            .build()
            .unwrap();
        let real_logs = messages
            .iter()
            .filter(|m| ["assistant", "user"].contains(&m.role.as_str()))
            .collect::<Vec<_>>();
        let sample_logs = messages
            .iter()
            .filter(|m| m.name.as_ref().map(|n| !n.is_empty()).unwrap_or_default())
            .collect::<Vec<_>>();
        let mut merged_logs = sample_logs
            .iter()
            .chain(real_logs.iter())
            .cloned()
            .cloned()
            .collect::<Vec<_>>();
        for m in &mut merged_logs {
            let name = m.name.as_ref().map(|n| n.as_str()).unwrap_or_default();
            m.customname = Some(
                ["assistant", "user"].contains(&m.role.as_str())
                    && m.name.is_some()
                    && !REPLACEMENT.contains_key(name),
            )
        }
        let s = self.0.as_ref();
        if !s.config.read().settings.xml_plot {
            // TODO: Non-xml plot
            // for (prev, next) in merged_logs.iter().zip(merged_logs.iter().skip(1)) {}
        }
        let mut last_assistant = real_logs
            .iter()
            .rfind(|m| m.role == "assistant" && m.merged.unwrap_or_default())
            .cloned()
            .cloned();
        if s.config.read().settings.strip_assistant {
            last_assistant.as_mut().map(|m| {
                m.strip = Some(true);
            });
        }
        let mut last_user = real_logs
            .iter()
            .rfind(|m| m.role == "user" && m.merged.unwrap_or_default())
            .cloned()
            .cloned();
        if s.config.read().settings.strip_human {
            last_user.as_mut().map(|m| {
                m.strip = Some(true);
            });
        }
        let mut system_messages = messages
            .iter()
            .filter(|m| m.role == "system" && m.name.is_none())
            .cloned()
            .collect::<Vec<_>>();
        let sys_messages_len = system_messages.len();
        for (i, m) in system_messages.iter_mut().enumerate() {
            if let Some(scenario) = re_scenario
                .captures(&m.content)
                .and_then(|c| c.get(1))
                .map(|c| c.as_str())
            {
                let re = RegexBuilder::new(r"{{scenario}}")
                    .multi_line(true)
                    .case_insensitive(true)
                    .build()
                    .unwrap();
                m.content = re
                    .replace_all(&s.config.read().scenario_format, scenario)
                    .to_string();
                m.scenario = Some(true);
            }
            let personalities = re_personality.captures(&m.content);
            if personalities.is_some() && personalities.as_ref().unwrap().len() == 3 {
                let re1 = RegexBuilder::new(r"{{char}}")
                    .multi_line(true)
                    .case_insensitive(true)
                    .build()
                    .unwrap();
                let re2 = RegexBuilder::new(r"{{personality}}")
                    .multi_line(true)
                    .case_insensitive(true)
                    .build()
                    .unwrap();
                let new_content = re1
                    .replace_all(
                        &s.config.read().personality_format,
                        &personalities.as_ref().unwrap()[1],
                    )
                    .to_string();
                m.content = re2
                    .replace_all(&new_content, &personalities.unwrap()[2])
                    .to_string();
                m.personality = Some(true);
            }
            m.main = if i == 0 { Some(true) } else { Some(false) };
            m.jailbreak = if i == sys_messages_len - 1 {
                Some(true)
            } else {
                Some(false)
            };
            if m.content.trim().is_empty() {
                m.discard = Some(true);
            }
        }
        // TODO: All sample
        // TODO: Non sample
        let systems: Vec<String> = Vec::new();
        if strategy.is_current() {
            // TODO: current chat
        }
        let prompt = messages
            .iter()
            .map_while(|m| self.generate_prompt(m))
            .collect::<Vec<_>>()
            .join("\n\n"); // TODO: Non xml plot is not
        (prompt, systems)
    }

    pub fn generate_prompt(&self, messages: &Message) -> Option<String> {
        if messages.merged.unwrap_or_default()
            || messages.discard.unwrap_or_default()
            || messages.content.is_empty()
        {
            return None;
        }
        let s = self.0.as_ref();
        if s.config.read().settings.xml_plot {
            let prefix = if *messages.customname.as_ref().unwrap_or(&false) {
                messages.role.clone()
                    + ": "
                    + messages
                        .name
                        .clone()
                        .unwrap_or_default()
                        .replace("_", " ")
                        .as_str()
                    + ": "
            } else if messages.role != "system"
                || messages
                    .name
                    .clone()
                    .map(|n| !n.is_empty())
                    .unwrap_or_default()
            {
                let replace = messages
                    .name
                    .clone()
                    .and_then(|n| REPLACEMENT.get(n.as_str()))
                    .or(REPLACEMENT.get(messages.role.as_str()))
                    .cloned()
                    .unwrap_or(&messages.role);
                format!("{}: ", replace)
            } else {
                let replace = REPLACEMENT
                    .get(messages.role.as_str())
                    .cloned()
                    .unwrap_or(&messages.role)
                    .to_string();
                format!("xmlPlot: {}", replace)
            };
            return Some(format!(
                "{}{}",
                if messages.strip.unwrap_or_default() {
                    String::new()
                } else {
                    prefix
                },
                messages.content
            ));
        } else {
            // TODO: Non-xml plot
        }
        None
    }

    pub fn xml_plot(&self, content: String, non_sys: Option<bool>) {
        let non_sys = non_sys.unwrap_or_default();
        self.0.regex_log.write().clear();
        let content = self.xml_plot_regex(content, 1);
        let merge_tag = MergeTag {
            all: !content.contains("<|Merge Disable|>"),
            system: !content.contains("<|Merge System Disable|>"),
            human: !content.contains("<|Merge Human Disable|>"),
            assistant: !content.contains("<|Merge Assistant Disable|>"),
        };
        let mut content = xml_plot_merge(&content, &merge_tag, non_sys);
        let mut split_content = {
            let re = Regex::new(r"\n\n(?=Assistant:|Human:)").unwrap();
            re.split(&content)
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        };
        let re = RegexBuilder::new(r"<@(\d+)>(.*?)</@\1>")
            .dot_matches_new_line(true)
            .build()
            .unwrap();
        while let Some(caps) = re.captures(&content) {
            let index = split_content.len() as isize - caps[1].parse::<isize>().unwrap() - 1;
            if index < 0 {
                warn!("{}", "Invalid index".yellow());
            } else {
                split_content[index as usize] +=
                    &("\n\n".to_string() + caps[2].to_string().as_str());
                content = content.replace(caps[0].to_string().as_str(), "");
            }
        }
        let content = split_content.join("\n\n");
        let re = Regex::new(r"(?s)<@(\d+)>.*?</@\1>").unwrap();
        let content = re.replace_all(&content, "").to_string();
        let content = self.xml_plot_regex(content, 2);
        let mut content = xml_plot_merge(&content, &merge_tag, non_sys);
        let split_human = content
            .split("\n\nHuman:")
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        // TODO: handle api key
        if split_human.len() > 2
            && split_content
                .last()
                .unwrap()
                .contains("<|Plain Prompt Enable|>")
            && !content.contains("\n\nPlainPrompt:")
        {
            let split = split_human
                .iter()
                .rev()
                .skip(1)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n\nHuman:");
            let re = Regex::new(r"\n\nHuman: *PlainPrompt:").unwrap();
            content = split.clone()
                + "\n\nPlainPrompt:"
                + re.replace_all(split.as_str(), "\n\nPlainPrompt:")
                    .to_string()
                    .as_str();
        }
        let c = self.xml_plot_regex(content, 3);
        let re1 = Regex::new(r"(?m)<regex( +order *= *\d)?>.*?</regex>").unwrap();
        let re2 = Regex::new(r"(?m)\r\n|\r").unwrap();
        let re3 = Regex::new(r"\s*<\|curtail\|>\s*").unwrap();
        let re4 = Regex::new(r"\s*<\|join\|>\s*").unwrap();
        let re5 = Regex::new(r"\s*<\|space\|>\s*").unwrap();
        let re6 = Regex::new(r"\s*\n\n(H(uman)?|A(ssistant)?): +").unwrap();
        let re7 = Regex::new(r"<\|(\\.*?)\|>").unwrap();
        let replacer = |caps: &regex::Captures| {
            let re = regex::Regex::new(r##"\\?""##).unwrap();
            let Some(p1) = caps.get(1).map(|o| o.as_str()) else {
                return caps[0].to_string();
            };
            let p1 = re.replace_all(p1, "\\\"").to_string();
            let Ok(json) = serde_json::from_str::<Value>(p1.as_str()) else {
                return caps[0].to_string();
            };
            return json.to_string();
        };
        let content = re1.replace_all(c.as_str(), "").to_string();
        let content = re2.replace_all(content.as_str(), "\n").to_string();
        let content = re3.replace_all(content.as_str(), "\n").to_string();
        let content = re4.replace_all(content.as_str(), "").to_string();
        let content = re5.replace_all(content.as_str(), " ").to_string();
        let content = re6.replace_all(content.as_str(), "\n\n$1: ").to_string();
        let content = re7.replace_all(content.as_str(), replacer).to_string();
        // TODO: api key logic
        let re1 = Regex::new(r"\s*<\|(?!padtxt).*?\|>\s*").unwrap();
        let re2 = Regex::new(r"\s*<\|.*?\|>\s*").unwrap();
        let re3 = Regex::new(r"^Human: *|\n\nAssistant: *$").unwrap();
        let re4 = Regex::new(r"(?<=\n)\n(?=\n)").unwrap();
        let c = if !self.0.config.read().settings.padtxt.is_empty() {
            re1.replace_all(content.as_str(), "\n\n").to_string()
        } else {
            re2.replace_all(content.as_str(), "\n\n").to_string()
        }
        .trim()
        .to_string();
        let c = re3.replace_all(c.as_str(), "").to_string();
        re4.replace_all(c.as_str(), "").to_string();
    }

    fn xml_plot_regex(&self, mut content: String, order: i64) -> String {
        let re = RegexBuilder::new(
            format!(
                "<regex(?: +order *= *{}){}> *\"(/?)(.*)\\1(.*?)\" *: *\"(.*?)\" *</regex>",
                order,
                if order == 2 { "?" } else { "" }
            )
            .as_str(),
        )
        .multi_line(true)
        .build()
        .unwrap();
        let res = re
            .find_iter(content.as_ref())
            .map(|m| m.as_str().to_string())
            .collect::<Vec<_>>();
        for m in res {
            let re = Regex::new(
                r##"<regex(?: +order *= *\d)?> *"(/?)(.*)\1(.*?)" *: *"(.*?)" *</regex>"##,
            )
            .unwrap();
            if let Some(caps) = re.captures(m.as_str()) {
                if caps.len() < 5 {
                    warn!("{}", "Regex capture group is less than 5".yellow());
                    continue;
                }
                *self.0.regex_log.write() += "\n";
                let ecma_re = caps[2].to_string();
                let flags = caps[3].to_string();
                let to = caps[4].to_string();
                let Ok(ecma_re) = regress::Regex::with_flags(&ecma_re, flags.as_str()) else {
                    warn!("{}", "ECMA regex is invalid".yellow());
                    continue;
                };
                let to = to.replace("\\?\"", "\\\"')}\"");
                content = ecma_re.replace_all(content.as_ref(), to.as_str()).into();
            }
        }
        content
    }
}

trait Replace<'h> {
    fn replace_all(&self, content: &'h str, to: &str) -> Cow<'h, str>;
}

impl<'h> Replace<'h> for regress::Regex {
    fn replace_all(&self, content: &'h str, to: &str) -> Cow<'h, str> {
        let mut new_content = Cow::from(content);
        // find capture groups in to
        let re = regex::Regex::new(r"\$([1-9]+\d*)").unwrap();
        let to = to.to_string();
        let captures = re
            .find_iter(to.as_str())
            .map_while(|m| m.as_str()[1..].parse::<usize>().ok())
            .collect::<Vec<_>>();
        // find all matches in content
        let mut offset: isize = 0;
        for m in self.find_iter(content) {
            let range_len = m.range().end - m.range().start;
            let range = (m.range().start as isize + offset) as usize
                ..(m.range().end as isize + offset) as usize;
            let grp = m.captures;
            if grp.is_empty() {
                continue;
            }
            let mut to = to.clone();
            for cap in &captures {
                if cap == &0 {
                    continue;
                }
                if let Some(capture) = grp.get(*cap - 1).and_then(|o| o.clone()) {
                    let capture_str = content[capture].to_string();
                    to = to.replace(&format!("${}", cap), &capture_str);
                }
            }
            new_content.to_mut().replace_range(range.clone(), &to);
            offset += to.len() as isize - range_len as isize;
        }
        new_content
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_replace_all() {
        let re = regress::Regex::with_flags(r"(\d+)", "gs").unwrap();
        let content = "123 456 789";
        let to = "$1kkk";
        let result = re.replace_all(content, to);
        assert_eq!(result, "123kkk 456kkk 789kkk");
    }
}

fn xml_plot_merge(content: &str, merge_tag: &MergeTag, non_sys: bool) -> String {
    let re_check = regex::Regex::new(r"(\n\n|^\s*)xmlPlot:\s*").unwrap();
    let mut content = content.to_string();

    if re_check.is_match(&content) {
        if !non_sys {
            let re_remove = regress::Regex::with_flags(
                r"(\n\n|^\s*)(?<!\n\n(Human|Assistant):.*?)xmlPlot:\s*",
                "s",
            )
            .unwrap();
            content = re_remove.replace_all(&content, "$1").to_string();
        }
        let re = regex::Regex::new(r"(\n\n|^\s*)xmlPlot: *").unwrap();
        let to = if merge_tag.system && merge_tag.human && merge_tag.all {
            "\n\nHuman: "
        } else {
            "$1"
        };
        content = re.replace_all(&content, to).to_string();
    }

    if merge_tag.all && merge_tag.human {
        let re = RegexBuilder::new(r"(?:\n\n|^\s*)Human:(.*?(?:\n\nAssistant:|$))")
            .dot_matches_new_line(true)
            .build()
            .unwrap();
        let replacer = |caps: &regex::Captures| {
            let re = regex::Regex::new(r"\n\nHuman:\s*").unwrap();
            if caps.len() < 2 {
                return caps[0].to_string();
            }
            let p1 = caps.get(1).unwrap().as_str();
            let p1 = re.replace_all(p1, "\n\n");
            return format!("\n\nHuman:{}", p1);
        };
        content = re.replace_all(&content, replacer).to_string();
    }
    if merge_tag.all && merge_tag.assistant {
        let re = RegexBuilder::new(r"\n\nAssistant:(.*?(?:\n\nHuman:|$))")
            .dot_matches_new_line(true)
            .build()
            .unwrap();
        let replacer = |caps: &regex::Captures| {
            let re = regex::Regex::new(r"\n\nAssistant:\s*").unwrap();
            if caps.len() < 2 {
                return caps[0].to_string();
            }
            let p1 = caps.get(1).unwrap().as_str();
            let p1 = re.replace_all(p1, "\n\n");
            return format!("\n\nAssistant:{}", p1);
        };
        content = re.replace_all(&content, replacer).to_string();
    }
    content
}

struct MergeTag {
    all: bool,
    system: bool,
    human: bool,
    assistant: bool,
}
