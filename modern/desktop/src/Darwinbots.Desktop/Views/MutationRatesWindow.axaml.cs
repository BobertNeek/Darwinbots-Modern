using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class MutationRatesWindow : Window
{
    private static readonly string[] ClassNames =
    [
        "Point", "Point2", "Delta", "Copy Error", "Copy Error2", "Insertion", "Reversal",
        "Minor Deletion", "Major Deletion", "Amplification", "Translocation",
    ];

    private MutationProfileUpdate _profile;
    private bool _loading;

    public bool Accepted { get; private set; }
    public MutationProfileUpdate Profile => _profile;

    public MutationRatesWindow() : this(MutationProfileUpdate.Default) { }

    public MutationRatesWindow(MutationProfileUpdate profile)
    {
        _profile = profile ?? throw new ArgumentNullException(nameof(profile));
        InitializeComponent();
        _loading = true;
        ClassSelector.ItemsSource = ClassNames;
        ClassSelector.SelectedIndex = 0;
        LoadClass();
        WireChanges();
    }

    private void WireChanges()
    {
        foreach (var numeric in new[] { OneIn, Mean, StdDev, NumberCategory, DereferenceCategory, OperatorCategory, LogicCategory, ComparisonCategory, FlowCategory })
            numeric.ValueChanged += (_, _) => UpdateSummary();
        Preference.PropertyChanged += (_, _) => UpdateSummary();
        Enabled.IsCheckedChanged += (_, _) => UpdateSummary();
    }

    private void ClassSelector_SelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (!_loading)
        {
            SaveClass();
            LoadClass();
        }
    }

    private void LoadClass()
    {
        _loading = true;
        var index = Math.Clamp(ClassSelector.SelectedIndex, 0, ClassNames.Length - 1);
        var rate = Rate(index);
        Enabled.IsChecked = rate.OneIn > 0 && rate.OneIn < 1e300;
        OneIn.Value = (decimal)Math.Min(rate.OneIn, 1e18);
        Mean.Value = (decimal)rate.Mean;
        StdDev.Value = (decimal)rate.StdDev;
        var preference = index switch
        {
            0 or 1 => _profile.PointValueChangePercent,
            3 or 4 => _profile.CopyErrorValueChangePercent,
            _ => 80,
        };
        PreferencePanel.IsVisible = index is 0 or 1 or 3 or 4;
        Preference.Value = preference;
        PreferenceValue.Text = $"{preference:0}% value change preference";
        var categories = index is 0 or 1 ? _profile.PointCategoryDistribution : _profile.CopyErrorCategoryDistribution;
        CategoryPanel.IsVisible = index is 0 or 1 or 3 or 4;
        NumberCategory.Value = (decimal)categories.Number;
        DereferenceCategory.Value = (decimal)categories.Dereference;
        OperatorCategory.Value = (decimal)categories.Operator;
        LogicCategory.Value = (decimal)categories.Logic;
        ComparisonCategory.Value = (decimal)categories.Comparison;
        FlowCategory.Value = (decimal)categories.Flow;
        ClassDescription.Text = index switch
        {
            0 => "Life mutation. Changes a numeric token while preserving DNA structure.",
            1 => "Life mutation. Replaces a token with a category-weighted outcome.",
            2 => "Life mutation. Evolves one inherited mutation rate.",
            3 or 4 => "Reproduction mutation. Replaces copied DNA with a category-weighted outcome.",
            5 => "Reproduction mutation. Inserts a new token block.",
            6 => "Reproduction mutation. Reverses a sampled DNA segment.",
            7 or 8 => "Reproduction mutation. Deletes a sampled DNA segment.",
            9 => "Reproduction mutation. Amplifies a sampled DNA segment.",
            _ => "Reproduction mutation. Moves a sampled DNA segment.",
        };
        _loading = false;
        UpdateSummary();
    }

    private void SaveClass()
    {
        if (_loading) return;
        var index = Math.Clamp(ClassSelector.SelectedIndex, 0, ClassNames.Length - 1);
        var oneIn = Enabled.IsChecked == true ? Math.Max((double)(OneIn.Value ?? 1m), 1) : 0;
        var rate = new MutationRateUpdate(oneIn, (double)(Mean.Value ?? 0m), Math.Max((double)(StdDev.Value ?? 0m), 0));
        _profile = SetRate(index, rate);
        var preference = (byte)Math.Clamp(Preference.Value, 0, 100);
        if (index is 0 or 1)
            _profile = _profile with { PointValueChangePercent = preference };
        else if (index is 3 or 4)
            _profile = _profile with { CopyErrorValueChangePercent = preference };
        var categories = new MutationCategoryDistributionUpdate(
            Math.Max((double)(NumberCategory.Value ?? 0m), 0),
            Math.Max((double)(DereferenceCategory.Value ?? 0m), 0),
            Math.Max((double)(OperatorCategory.Value ?? 0m), 0),
            Math.Max((double)(LogicCategory.Value ?? 0m), 0),
            Math.Max((double)(ComparisonCategory.Value ?? 0m), 0),
            Math.Max((double)(FlowCategory.Value ?? 0m), 0));
        if (index is 0 or 1)
            _profile = _profile with { PointCategoryDistribution = categories };
        else if (index is 3 or 4)
            _profile = _profile with { CopyErrorCategoryDistribution = categories };
    }

    private MutationRateUpdate Rate(int index) => index switch
    {
        0 => _profile.Point,
        1 => _profile.Point2,
        2 => _profile.Delta,
        3 => _profile.CopyError,
        4 => _profile.CopyError2,
        5 => _profile.Insertion,
        6 => _profile.Reversal,
        7 => _profile.MinorDeletion,
        8 => _profile.MajorDeletion,
        9 => _profile.Amplification,
        _ => _profile.Translocation,
    };

    private MutationProfileUpdate SetRate(int index, MutationRateUpdate rate) => index switch
    {
        0 => _profile with { Point = rate },
        1 => _profile with { Point2 = rate },
        2 => _profile with { Delta = rate },
        3 => _profile with { CopyError = rate },
        4 => _profile with { CopyError2 = rate },
        5 => _profile with { Insertion = rate },
        6 => _profile with { Reversal = rate },
        7 => _profile with { MinorDeletion = rate },
        8 => _profile with { MajorDeletion = rate },
        9 => _profile with { Amplification = rate },
        _ => _profile with { Translocation = rate },
    };

    private void UpdateSummary()
    {
        if (_loading) return;
        var total = (double)(NumberCategory.Value ?? 0) + (double)(DereferenceCategory.Value ?? 0) +
                    (double)(OperatorCategory.Value ?? 0) + (double)(LogicCategory.Value ?? 0) +
                    (double)(ComparisonCategory.Value ?? 0) + (double)(FlowCategory.Value ?? 0);
        CategoryTotal.Text = $"TOTAL {total:0.##}%";
        var oneIn = Math.Max((double)(OneIn.Value ?? 1), 1);
        Summary.Text = Enabled.IsChecked == true
            ? $"{ClassNames[Math.Clamp(ClassSelector.SelectedIndex, 0, ClassNames.Length - 1)]}: estimated probability 1 in {oneIn:0.###}."
            : "This mutation class is disabled.";
    }

    private void Default_Click(object? sender, RoutedEventArgs e)
    {
        SaveClass();
        _profile = MutationProfileUpdate.Default;
        LoadClass();
    }

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        SaveClass();
        Accepted = true;
        Close();
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
