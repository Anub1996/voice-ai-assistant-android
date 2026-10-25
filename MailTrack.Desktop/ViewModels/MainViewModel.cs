using System;
using System.Collections.ObjectModel;
using System.Linq;
using System.Windows;
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using MailTrack.Desktop.Data;
using MailTrack.Desktop.Models;
using MailTrack.Desktop.Services;

namespace MailTrack.Desktop.ViewModels
{
    public partial class MainViewModel : ObservableObject
    {
        private readonly DatabaseManager _db = new();
        private readonly ImapMonitorService _monitor;
        private readonly DispatcherTimer _refreshTimer;

        public ObservableCollection<TrackedEmail> Emails { get; } = new();
        public ObservableCollection<DlpRule> Rules { get; } = new();
        public ObservableCollection<Alert> Alerts { get; } = new();
        public ObservableCollection<MonitoredAccount> Accounts { get; } = new();

        [ObservableProperty] private string _statusText = "Ready";
        [ObservableProperty] private string _searchQuery = "";
        [ObservableProperty] private string _currentFilter = "all";
        [ObservableProperty] private int _totalCount;
        [ObservableProperty] private int _incomingCount;
        [ObservableProperty] private int _outgoingCount;
        [ObservableProperty] private int _alertCount;
        [ObservableProperty] private int _activeRuleCount;
        [ObservableProperty] private int _pendingAlertCount;
        [ObservableProperty] private bool _isMonitoring;
        [ObservableProperty] private TrackedEmail? _selectedEmail;
        [ObservableProperty] private int _selectedTabIndex;

        public MainViewModel()
        {
            _monitor = new ImapMonitorService(_db);
            _monitor.EmailCaptured += OnEmailCaptured;
            _monitor.AlertRaised += OnAlertRaised;
            _monitor.StatusChanged += s => Application.Current.Dispatcher.Invoke(() => StatusText = s);

            _refreshTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(5) };
            _refreshTimer.Tick += (_, _) => RefreshStats();

            LoadAll();
            _refreshTimer.Start();
        }

        private void LoadAll()
        {
            RefreshEmails();
            RefreshRules();
            RefreshAlerts();
            RefreshAccounts();
            RefreshStats();
        }

        [RelayCommand]
        private void RefreshEmails()
        {
            string? direction = CurrentFilter switch
            {
                "incoming" => "incoming",
                "outgoing" => "outgoing",
                _ => null
            };
            bool? alertsOnly = CurrentFilter == "alerts" ? true : null;
            string? search = string.IsNullOrWhiteSpace(SearchQuery) ? null : SearchQuery;

            var list = _db.GetEmails(direction, search, alertsOnly);
            Emails.Clear();
            foreach (var e in list) Emails.Add(e);
        }

        [RelayCommand]
        private void RefreshRules()
        {
            var list = _db.GetAllRules();
            Rules.Clear();
            foreach (var r in list) Rules.Add(r);
        }

        [RelayCommand]
        private void RefreshAlerts()
        {
            var list = _db.GetAlerts(unacknowledgedOnly: true);
            Alerts.Clear();
            foreach (var a in list) Alerts.Add(a);
        }

        private void RefreshAccounts()
        {
            var list = _db.GetAccounts();
            Accounts.Clear();
            foreach (var a in list) Accounts.Add(a);
        }

        private void RefreshStats()
        {
            var stats = _db.GetStats();
            TotalCount = stats.total;
            IncomingCount = stats.incoming;
            OutgoingCount = stats.outgoing;
            AlertCount = stats.alerts;
            ActiveRuleCount = stats.activeRules;
            PendingAlertCount = stats.pendingAlerts;
        }

        [RelayCommand]
        private void SetFilter(string filter)
        {
            CurrentFilter = filter;
            RefreshEmails();
        }

        [RelayCommand]
        private void Search()
        {
            RefreshEmails();
        }

        [RelayCommand]
        private void ToggleFlag(TrackedEmail email)
        {
            _db.ToggleEmailFlag(email.Id);
            RefreshEmails();
        }

        [RelayCommand]
        private void AcknowledgeAlert(Alert alert)
        {
            _db.AcknowledgeAlert(alert.Id);
            RefreshAlerts();
            RefreshStats();
        }

        [RelayCommand]
        private void AcknowledgeAllAlerts()
        {
            _db.AcknowledgeAllAlerts();
            RefreshAlerts();
            RefreshStats();
        }

        [RelayCommand]
        private void AddRule(DlpRule rule)
        {
            _db.InsertRule(rule);
            RefreshRules();
            RefreshStats();
        }

        [RelayCommand]
        private void UpdateRule(DlpRule rule)
        {
            _db.UpdateRule(rule);
            RefreshRules();
        }

        [RelayCommand]
        private void DeleteRule(DlpRule rule)
        {
            _db.DeleteRule(rule.Id);
            RefreshRules();
            RefreshStats();
        }

        [RelayCommand]
        private void ToggleRuleEnabled(DlpRule rule)
        {
            rule.Enabled = !rule.Enabled;
            _db.UpdateRule(rule);
            RefreshRules();
            RefreshStats();
        }

        [RelayCommand]
        private void AddAccount(MonitoredAccount account)
        {
            _db.InsertAccount(account);
            RefreshAccounts();
        }

        [RelayCommand]
        private void RemoveAccount(MonitoredAccount account)
        {
            _monitor.StopMonitoring(account.Id);
            _db.DeleteAccount(account.Id);
            RefreshAccounts();
        }

        [RelayCommand]
        private void StartMonitoring()
        {
            foreach (var account in Accounts.Where(a => a.Enabled))
                _monitor.StartMonitoring(account);
            IsMonitoring = true;
            StatusText = "Monitoring started";
        }

        [RelayCommand]
        private void StopMonitoring()
        {
            _monitor.StopAll();
            IsMonitoring = false;
            StatusText = "Monitoring stopped";
        }

        private void OnEmailCaptured(TrackedEmail email)
        {
            Application.Current.Dispatcher.Invoke(() =>
            {
                Emails.Insert(0, email);
                if (Emails.Count > 500) Emails.RemoveAt(Emails.Count - 1);
                RefreshStats();
            });
        }

        private void OnAlertRaised(Alert alert)
        {
            Application.Current.Dispatcher.Invoke(() =>
            {
                Alerts.Insert(0, alert);
                RefreshStats();
            });
        }
    }
}
