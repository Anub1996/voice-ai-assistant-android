using System;
using System.Globalization;
using System.Windows;
using System.Windows.Data;
using System.Windows.Media;

namespace MailTrack.Desktop.Converters
{
    public class BoolToVisibilityConverter : IValueConverter
    {
        public object Convert(object value, Type t, object p, CultureInfo c)
            => value is true ? Visibility.Visible : Visibility.Collapsed;
        public object ConvertBack(object value, Type t, object p, CultureInfo c)
            => value is Visibility.Visible;
    }

    public class InverseBoolToVisibilityConverter : IValueConverter
    {
        public object Convert(object value, Type t, object p, CultureInfo c)
            => value is true ? Visibility.Collapsed : Visibility.Visible;
        public object ConvertBack(object value, Type t, object p, CultureInfo c)
            => value is Visibility.Collapsed;
    }

    public class SeverityToBrushConverter : IValueConverter
    {
        public object Convert(object value, Type t, object p, CultureInfo c)
        {
            return value?.ToString()?.ToLower() switch
            {
                "critical" => new SolidColorBrush(Color.FromRgb(183, 28, 28)),
                "high" => new SolidColorBrush(Color.FromRgb(244, 67, 54)),
                "medium" => new SolidColorBrush(Color.FromRgb(255, 152, 0)),
                "low" => new SolidColorBrush(Color.FromRgb(76, 175, 80)),
                _ => new SolidColorBrush(Colors.Gray)
            };
        }
        public object ConvertBack(object value, Type t, object p, CultureInfo c) => "";
    }

    public class DirectionToColorConverter : IValueConverter
    {
        public object Convert(object value, Type t, object p, CultureInfo c)
        {
            return value?.ToString() == "incoming"
                ? new SolidColorBrush(Color.FromRgb(76, 175, 80))
                : new SolidColorBrush(Color.FromRgb(33, 150, 243));
        }
        public object ConvertBack(object value, Type t, object p, CultureInfo c) => "";
    }

    public class TimestampConverter : IValueConverter
    {
        public object Convert(object value, Type t, object p, CultureInfo c)
        {
            if (value is not DateTime dt) return "";
            var diff = DateTime.UtcNow - dt;
            if (diff.TotalMinutes < 1) return "Just now";
            if (diff.TotalHours < 1) return $"{(int)diff.TotalMinutes}m ago";
            if (diff.TotalDays < 1) return $"{(int)diff.TotalHours}h ago";
            return dt.ToLocalTime().ToString("MMM d, HH:mm");
        }
        public object ConvertBack(object value, Type t, object p, CultureInfo c) => DateTime.MinValue;
    }

    public class UpperCaseConverter : IValueConverter
    {
        public object Convert(object value, Type t, object p, CultureInfo c)
            => value?.ToString()?.ToUpper() ?? "";
        public object ConvertBack(object value, Type t, object p, CultureInfo c)
            => value;
    }

    public class FilterToBoolConverter : IValueConverter
    {
        public object Convert(object value, Type t, object parameter, CultureInfo c)
            => value?.ToString() == parameter?.ToString();
        public object ConvertBack(object value, Type t, object parameter, CultureInfo c)
            => parameter?.ToString() ?? "all";
    }
}
