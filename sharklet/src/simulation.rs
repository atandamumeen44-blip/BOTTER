
Commit changes
There was an error committing your changes: atandamumeen44-blip has committed since you started editing. See what changed
Commit message
Update simulation.rs
Extended description
Add an optional extended description...
Direct commit or PR

Commit directly to the main branch

Create a new branch for this commit and start a pull request Learn more about pull requests
Skip to content
atandamumeen44-blip
BOTTER
Repository navigation
Code
Issues
3
 (3)
Pull requests
Actions
Projects
Wiki
Security and quality
Insights
Settings
Files
Go to file
t
T
sharklet
config
contracts
dashboard
src
api.rs
dex_registry.rs
executor.rs
gas_manager.rs
logger.rs
main.rs
price_oracle.rs
profit_calc.rs
risk_engine.rs
rpc_manager.rs
scanner.rs
simulation.rs
Cargo.toml
README.md
blueprint.html
dashboard.html
filestructure.txt
run_bot.sh
rust-toolchain.toml.txt
.env.example
.gitignore
Cargo.toml
README.md
blueprint.html
dashboard.html
filestructure.txt
run_bot.sh
BOTTER/sharklet/src
/
simulation.rs
in
main

Edit

Preview
Indent mode

Spaces
Indent size

4
Line wrap mode

No wrap
Editing simulation.rs file contents
119
120
121
122
123
124
125
126
127
128
129
130
131
132
133
134
135
136
137
138
139
140
141
142
143
144
145
146
147
148
149
150
151
152
153
154
155
156
157
158
159
160
161
162
163
164
165
166
167
168
169

            let profit = sized.net_profit_usd; // using net_profit_usd
            estimated_profit_after_gas_usd = Some(profit);
            if profit < self.config.min_profit_after_gas_usd {
                trace.push(("profit floor".into(), CheckStatus::Fail(format!("profit ${:.2} < min ${:.2}", profit, self.config.min_profit_after_gas_usd))));
                passed = false;
            }
        } else {
            trace.push(("eth_call simulation".into(), CheckStatus::Skipped("earlier check failed".into())));
        }

        SimulationReport {
            passed,
            trace,
            gas_metrics: gas_metrics.unwrap_or(GasMetrics { base_fee_gwei: 0.0, priority_fee_gwei: 0.0, max_total_fee_gwei: 0.0, estimated_gas_cost_usd: 0.0 }),
            rpc_consensus_ok: rpc_ok,
            predicted_amount_out,
            estimated_gas_units,
            estimated_profit_after_gas_usd,
        }
    }

    async fn check_rpc_health(&self, trace: &mut Vec<(String, CheckStatus)>) -> (bool, Option<Block<TxHash>>) {
        let consensus = match self.rpc_manager.consensus_block_number().await {
            Some(pair) => { trace.push(("rpc consensus".into(), CheckStatus::Pass(format!("blocks {}/{}", pair.a, pair.b)))); true }
            None => { trace.push(("rpc consensus".into(), CheckStatus::Fail("disagree".into()))); false }
        };
        (consensus, None)
    }

    async fn check_gas(&self, trace: &mut Vec<(String, CheckStatus)>) -> (bool, Option<GasMetrics>) {
        let gas_price = self.provider.get_gas_price().await.unwrap_or_default();
        let gwei = gas_price.as_u128() as f64 / 1e9;
        if gwei > self.config.max_total_fee_gwei {
            trace.push(("gas ceiling".into(), CheckStatus::Fail(format!("{:.1} gwei", gwei))));
            (false, None)
        } else {
            trace.push(("gas ceiling".into(), CheckStatus::Pass(format!("{:.1} gwei", gwei))));
            (true, Some(GasMetrics { base_fee_gwei: gwei, priority_fee_gwei: 0.0, max_total_fee_gwei: gwei, estimated_gas_cost_usd: 0.0 }))
        }
    }

    fn check_liquidity_and_sizing(&self, opp: &Opportunity, sized: &SizedTrade, trace: &mut Vec<(String, CheckStatus)>) {
        let depth = opp.buy_pool_depth.min(opp.sell_pool_depth);
        if depth < self.config.min_liquidity_usd {
            trace.push(("liquidity".into(), CheckStatus::Fail(format!("${:.0}", depth))));
        } else {
            trace.push(("liquidity".into(), CheckStatus::Pass(format!("${:.0}", depth))));
        }
    }
}
Use Control + Shift + m to toggle the tab key moving focus. Alternatively, use esc then tab to move to the next interactive element on the page.
Editing BOTTER/sharklet/src/simulation.rs at main · atandamumeen44-blip/BOTTER
