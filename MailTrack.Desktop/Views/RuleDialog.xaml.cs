using System.Windows;
using MailTrack.Desktop.Models;

namespace MailTrack.Desktop.Views
{
    public partial class RuleDialog : Window
    {
        public DlpRule? Result { get; private set; }

        public RuleDialog()
        {
            InitializeComponent();
        }

        private void Save_Click(object sender, RoutedEventArgs e)
        {
            if (string.IsNullOrWhiteSpace(TxtName.Text))
            {
                MessageBox.Show("Rule name is required.", "Validation", MessageBoxButton.OK, MessageBoxImage.Warning);
                return;
            }
            if (string.IsNullOrWhiteSpace(TxtPattern.Text))
            {
                MessageBox.Show("Pattern / value is required.", "Validation", MessageBoxButton.OK, MessageBoxImage.Warning);
                return;
            }

            Result = new DlpRule
            {
                Name = TxtName.Text.Trim(),
                Description = TxtDescription.Text.Trim(),
                RuleType = ((FrameworkElement)CmbRuleType.SelectedItem).Tag.ToString()!,
                Pattern = TxtPattern.Text.Trim(),
                Direction = ((FrameworkElement)CmbDirection.SelectedItem).Tag.ToString()!,
                Severity = ((FrameworkElement)CmbSeverity.SelectedItem).Tag.ToString()!,
                Action = ((FrameworkElement)CmbAction.SelectedItem).Tag.ToString()!,
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
