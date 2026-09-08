# Integrate RuleEngine with Event Dispatcher and Reload Command

Status: resolved

## Overview
Update `CoreEventBus`, `Data` state, and `/reload_modules` command to use `RuleEngine`.

## Details
- Update `Data::rhai_manager` to store `Arc<dyn RuleEngine>` or `Arc<RhaiRuleEngine>`.
- Update `CoreEventBus::dispatch_message` to evaluate rules and handle `FlagSuggestion`.
- Update `reload_modules` command to trigger `RuleEngine::reload`.
