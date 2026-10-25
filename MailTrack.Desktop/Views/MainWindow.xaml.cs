using System.Windows;
using MailTrack.Desktop.Models;
using MailTrack.Desktop.ViewModels;

namespace MailTrack.Desktop.Views
{
    public partial class MainWindow : Window
    {
        private MainViewModel ViewModel => (MainViewModel)DataContext;

        public MainWindow()
        {
            InitializeComponent();
        }

        private void AddRule_Click(object sender, RoutedEventArgs e)
        {
            var dialog = new RuleDialog { Owner = this };
            if (dialog.ShowDialog() == true && dialog.Result != null)
                ViewModel.AddRuleCommand.Execute(dialog.Result);
        }

        private void AddAccount_Click(object sender, RoutedEventArgs e)
        {
            var dialog = new AccountDialog { Owner = this };
            if (dialog.ShowDialog() == true && dialog.Result != null)
                ViewModel.AddAccountCommand.Execute(dialog.Result);
        }

        protected override void OnClosing(System.ComponentModel.CancelEventArgs e)
        {
            ViewModel.StopMonitoringCommand.Execute(null);
            base.OnClosing(e);
        }
    }
}
