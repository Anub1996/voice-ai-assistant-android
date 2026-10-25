package com.voiceai.mailtrack.ui.ruleconfig

import android.os.Bundle
import android.view.View
import android.widget.ArrayAdapter
import android.widget.EditText
import android.widget.Spinner
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.google.android.material.floatingactionbutton.FloatingActionButton
import com.google.android.material.snackbar.Snackbar
import com.voiceai.mailtrack.MailTrackApp
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.data.entity.DlpRule
import com.voiceai.mailtrack.data.repository.MailTrackRepository
import com.voiceai.mailtrack.ui.adapter.RuleAdapter
import kotlinx.coroutines.launch

class RuleConfigActivity : AppCompatActivity() {

    private lateinit var repository: MailTrackRepository
    private lateinit var adapter: RuleAdapter

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_rule_config)
        setSupportActionBar(findViewById(R.id.toolbar))
        supportActionBar?.setDisplayHomeAsUpEnabled(true)
        supportActionBar?.title = "DLP Rules"

        val db = MailTrackApp.instance.database
        repository = MailTrackRepository(db.emailDao(), db.ruleDao(), db.alertDao())

        adapter = RuleAdapter(
            onToggle = { rule, enabled -> toggleRule(rule, enabled) },
            onEdit = { rule -> showRuleDialog(rule) },
            onDelete = { rule -> deleteRule(rule) }
        )

        val recyclerView = findViewById<RecyclerView>(R.id.recycler_rules)
        recyclerView.layoutManager = LinearLayoutManager(this)
        recyclerView.adapter = adapter

        repository.allRules.observe(this) { rules ->
            adapter.submitList(rules)
            findViewById<View>(R.id.tv_no_rules).visibility =
                if (rules.isEmpty()) View.VISIBLE else View.GONE
        }

        findViewById<FloatingActionButton>(R.id.fab_add_rule).setOnClickListener {
            showRuleDialog(null)
        }
    }

    private fun showRuleDialog(existingRule: DlpRule?) {
        val dialogView = layoutInflater.inflate(R.layout.dialog_rule, null)
        val etName = dialogView.findViewById<EditText>(R.id.et_rule_name)
        val etDescription = dialogView.findViewById<EditText>(R.id.et_rule_description)
        val etPattern = dialogView.findViewById<EditText>(R.id.et_rule_pattern)
        val spinnerType = dialogView.findViewById<Spinner>(R.id.spinner_rule_type)
        val spinnerDirection = dialogView.findViewById<Spinner>(R.id.spinner_direction)
        val spinnerSeverity = dialogView.findViewById<Spinner>(R.id.spinner_severity)
        val spinnerAction = dialogView.findViewById<Spinner>(R.id.spinner_action)

        val types = arrayOf("keyword", "domain", "regex", "attachment_type", "recipient_count")
        val directions = arrayOf("both", "incoming", "outgoing")
        val severities = arrayOf("low", "medium", "high", "critical")
        val actions = arrayOf("alert", "flag", "block")

        spinnerType.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item, types)
        spinnerDirection.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item, directions)
        spinnerSeverity.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item, severities)
        spinnerAction.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item, actions)

        if (existingRule != null) {
            etName.setText(existingRule.name)
            etDescription.setText(existingRule.description)
            etPattern.setText(existingRule.pattern)
            spinnerType.setSelection(types.indexOf(existingRule.ruleType).coerceAtLeast(0))
            spinnerDirection.setSelection(directions.indexOf(existingRule.direction).coerceAtLeast(0))
            spinnerSeverity.setSelection(severities.indexOf(existingRule.severity).coerceAtLeast(0))
            spinnerAction.setSelection(actions.indexOf(existingRule.action).coerceAtLeast(0))
        }

        AlertDialog.Builder(this)
            .setTitle(if (existingRule != null) "Edit Rule" else "New DLP Rule")
            .setView(dialogView)
            .setPositiveButton("Save") { _, _ ->
                val name = etName.text.toString().trim()
                val pattern = etPattern.text.toString().trim()
                if (name.isBlank() || pattern.isBlank()) {
                    Snackbar.make(
                        findViewById(R.id.recycler_rules),
                        "Name and pattern are required",
                        Snackbar.LENGTH_SHORT
                    ).show()
                    return@setPositiveButton
                }

                val rule = DlpRule(
                    id = existingRule?.id ?: 0,
                    name = name,
                    description = etDescription.text.toString().trim(),
                    ruleType = spinnerType.selectedItem.toString(),
                    pattern = pattern,
                    direction = spinnerDirection.selectedItem.toString(),
                    severity = spinnerSeverity.selectedItem.toString(),
                    action = spinnerAction.selectedItem.toString(),
                    enabled = existingRule?.enabled ?: true,
                    createdAt = existingRule?.createdAt ?: System.currentTimeMillis()
                )

                lifecycleScope.launch {
                    if (existingRule != null) repository.updateRule(rule)
                    else repository.insertRule(rule)
                }
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun toggleRule(rule: DlpRule, enabled: Boolean) {
        lifecycleScope.launch { repository.updateRule(rule.copy(enabled = enabled)) }
    }

    private fun deleteRule(rule: DlpRule) {
        AlertDialog.Builder(this)
            .setTitle("Delete Rule")
            .setMessage("Delete '${rule.name}'?")
            .setPositiveButton("Delete") { _, _ ->
                lifecycleScope.launch { repository.deleteRule(rule) }
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    override fun onSupportNavigateUp(): Boolean {
        onBackPressedDispatcher.onBackPressed()
        return true
    }
}
