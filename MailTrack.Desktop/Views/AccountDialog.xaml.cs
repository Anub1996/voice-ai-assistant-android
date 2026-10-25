using System.Windows;
using MailTrack.Desktop.Models;

namespace MailTrack.Desktop.Views
{
    public partial class AccountDialog : Window
    {
        public MonitoredAccount? Result { get; private set; }

        public AccountDialog()
        {
            InitializeComponent();
        }

        private void Save_Click(object sender, RoutedEventArgs e)
        {
            if (string.IsNullOrWhiteSpace(TxtName.Text))
            {
                MessageBox.Show("Account name is required.", "Validation", MessageBoxButton.OK, MessageBoxImage.Warning);
                return;
            }
            if (string.IsNullOrWhiteSpace(TxtEmail.Text))
            {
                MessageBox.Show("Email address is required.", "Validation", MessageBoxButton.OK, MessageBoxImage.Warning);
                return;
            }
            if (string.IsNullOrWhiteSpace(TxtHost.Text))
            {
                MessageBox.Show("IMAP server is required.", "Validation", MessageBoxButton.OK, MessageBoxImage.Warning);
                return;
            }
            if (string.IsNullOrWhiteSpace(TxtPassword.Password))
            {
                MessageBox.Show("Password is required.", "Validation", MessageBoxButton.OK, MessageBoxImage.Warning);
                return;
            }

            if (!int.TryParse(TxtPort.Text, out int port) || port < 1 || port > 65535)
            {
                MessageBox.Show("Port must be a number between 1 and 65535.", "Validation", MessageBoxButton.OK, MessageBoxImage.Warning);
                return;
            }

            if (!int.TryParse(TxtPollInterval.Text, out int pollInterval) || pollInterval < 10)
            {
                MessageBox.Show("Poll interval must be at least 10 seconds.", "Validation", MessageBoxButton.OK, MessageBoxImage.Warning);
                return;
            }

            Result = new MonitoredAccount
            {
                Name = TxtName.Text.Trim(),
                Email = TxtEmail.Text.Trim(),
                Host = TxtHost.Text.Trim(),
                Port = port,
                Password = TxtPassword.Password,
                UseSsl = ChkSsl.IsChecked == true,
                PollIntervalSeconds = pollInterval,
                Enabled = true
            };

            DialogResult = true;
        }

        private void Cancel_Click(object sender, RoutedEventArgs e)
        {
            DialogResult = false;
        }
    }
}
