use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Alerts to be configured in the dashboard.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Alerts {
    alerts: Vec<Alert>,
}

impl Alerts {
    pub(crate) const fn new(alerts: Vec<Alert>) -> Self {
        Self { alerts }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) enum AlertSeverity {
    // Critical issues that demand immediate attention. These are high-impact incidents that
    // affect the system's availability.
    #[serde(rename = "p1")]
    // TODO(Tsabary): currently the `Sos` variant is used only in tests, and removing the
    // `#[cfg(test)]` attribute results in a compilation error. When needed in non-test setup,
    // remove the attribute.
    #[cfg(test)]
    Sos,
    // Standard alerts for production issues that require attention around the clock but are not
    // as time-sensitive as SOS alerts.
    #[serde(rename = "p2")]
    Regular,
    // Important alerts that do not require overnight attention. These are delayed during night
    // hours to reduce unnecessary off-hours noise.
    #[serde(rename = "p3")]
    DayOnly,
    // Alerts that are only triggered during official business hours. These do not trigger during
    // holidays.
    #[serde(rename = "p4")]
    WorkingHours,
    // Non-critical alerts, meant purely for information. These are not intended to wake anyone up
    // and are monitored only by the development team.
    #[serde(rename = "p5")]
    Informational,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) enum AlertComparisonOp {
    #[serde(rename = "gt")]
    GreaterThan,
    #[serde(rename = "lt")]
    LessThan,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AlertLogicalOp {
    And,
    // TODO(Tsabary): remove the `allow(dead_code)` once this variant is used.
    #[allow(dead_code)]
    Or,
}

/// Defines the condition to trigger the alert.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AlertCondition {
    // The comparison operator to use when comparing the expression to the value.
    pub(crate) comparison_op: AlertComparisonOp,
    // The value to compare the expression to.
    pub(crate) comparison_value: f64,
    // The logical operator between this condition and other conditions.
    // TODO(Yael): Consider moving this field to the be one per alert to avoid ambiguity when
    // trying to use a combination of `and` and `or` operators.
    pub(crate) logical_op: AlertLogicalOp,
}

impl Serialize for AlertCondition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AlertCondition", 4)?;

        state.serialize_field(
            "evaluator",
            &serde_json::json!({
                "params": [self.comparison_value],
                "type": self.comparison_op
            }),
        )?;

        state.serialize_field(
            "operator",
            &serde_json::json!({
                "type": self.logical_op
            }),
        )?;

        state.serialize_field(
            "reducer",
            &serde_json::json!({
                "params": [],
                "type": "avg"
            }),
        )?;

        state.serialize_field("type", "query")?;

        state.end()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AlertGroup {
    Batcher,
    Consensus,
    Gateway,
    HttpServer,
    L1GasPrice,
    L1Messages,
    Mempool,
    StateSync,
}

/// Describes the properties of an alert defined in grafana.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Alert {
    // The name of the alert.
    pub(crate) name: &'static str,
    // The title that will be displayed.
    pub(crate) title: &'static str,
    // The group that the alert will be displayed under.
    #[serde(rename = "ruleGroup")]
    pub(crate) alert_group: AlertGroup,
    // The expression to evaluate for the alert.
    pub(crate) expr: String,
    // The conditions that must be met for the alert to be triggered.
    pub(crate) conditions: &'static [AlertCondition],
    // The time duration for which the alert conditions must be true before an alert is triggered.
    #[serde(rename = "for")]
    pub(crate) pending_duration: &'static str,
    // The interval in sec between evaluations of the alert.
    #[serde(rename = "intervalSec")]
    pub(crate) evaluation_interval_sec: u64,
    // The severity level of the alert.
    pub(crate) severity: AlertSeverity,
}
