using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace Darwinbots.Desktop.Core;

public sealed record ContestCompetitorResult(
    string Id,
    string Name,
    int Wins,
    int Population,
    bool IsDisqualified = false);

public sealed record ContestResultSnapshot(
    string ContestName,
    int Round,
    int MinimumRounds,
    int MaximumRounds,
    IReadOnlyList<ContestCompetitorResult> Competitors)
{
    public static ContestResultSnapshot Empty { get; } = new("No local contest result", 0, 0, 0, []);

    public static ContestResultSnapshot FromSnapshot(EngineSnapshot snapshot)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        if (snapshot.LocalMode.Status == LocalModeStatus.Idle || snapshot.LocalMode.MaximumRounds == 0)
            return Empty;

        var winsBySpecies = snapshot.LocalMode.Results
            .Where(value => value.Winner is not null)
            .GroupBy(value => value.Winner!.Value)
            .ToDictionary(group => group.Key, group => group.Count());
        var competitors = snapshot.Species
            .Select((species, index) => new ContestCompetitorResult(
                $"species-{index}",
                species.Name,
                winsBySpecies.GetValueOrDefault((uint)index),
                snapshot.Organisms.Count(value => value.Species == index),
                false))
            .Where(value => value.Population > 0)
            .Take(5)
            .ToArray();
        return competitors.Length == 0
            ? Empty
            : new ContestResultSnapshot(
                $"Local {snapshot.LocalMode.Kind} result",
                (int)snapshot.LocalMode.CurrentRound,
                (int)snapshot.LocalMode.MinimumRounds,
                (int)snapshot.LocalMode.MaximumRounds,
                competitors);
    }
}

public enum ContestAdjudicationKind
{
    ManualWin,
    InstantWin,
}

public sealed record ContestAdjudication(
    ContestAdjudicationKind Kind,
    string ContestName,
    int Round,
    string CompetitorId);

public sealed class ContestCompetitorViewModel : INotifyPropertyChanged
{
    private int _wins;
    private bool _instantWinner;
    private string _status;

    public ContestCompetitorViewModel(ContestCompetitorResult result)
    {
        Id = result.Id;
        Name = result.Name;
        _wins = result.Wins;
        Population = result.Population;
        IsDisqualified = result.IsDisqualified;
        _status = result.IsDisqualified ? "DISQUALIFIED" : "READY";
    }

    public string Id { get; }
    public string Name { get; }
    public int Wins { get => _wins; private set => Set(ref _wins, value); }
    public int Population { get; }
    public bool IsDisqualified { get; }
    public bool InstantWinner { get => _instantWinner; private set => Set(ref _instantWinner, value); }
    public string Status { get => _status; private set => Set(ref _status, value); }
    public bool CanAdjudicate => !IsDisqualified && !InstantWinner;

    public event PropertyChangedEventHandler? PropertyChanged;

    internal void Record(ContestAdjudicationKind kind)
    {
        if (kind == ContestAdjudicationKind.ManualWin) Wins++;
        else
        {
            Wins++;
            InstantWinner = true;
        }
        Status = kind == ContestAdjudicationKind.InstantWin ? "INSTANT WIN" : "MANUAL WIN";
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CanAdjudicate)));
    }

    private void Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}

public sealed class ContestResultsViewModel : INotifyPropertyChanged
{
    private readonly Action<ContestAdjudication> _submit;
    private string _status;

    public ContestResultsViewModel(
        ContestResultSnapshot result,
        Action<ContestAdjudication>? submit = null)
    {
        ArgumentNullException.ThrowIfNull(result);
        ContestName = result.ContestName;
        Round = result.Round;
        MinimumRounds = result.MinimumRounds;
        MaximumRounds = result.MaximumRounds;
        Competitors = new ObservableCollection<ContestCompetitorViewModel>(
            result.Competitors.Select(value => new ContestCompetitorViewModel(value)));
        _submit = submit ?? (_ => { });
        _status = Competitors.Count == 0 ? "NO LOCAL CONTEST RESULT" : "READY FOR LOCAL ADJUDICATION";
    }

    public string ContestName { get; }
    public int Round { get; }
    public int MinimumRounds { get; }
    public int MaximumRounds { get; }
    public bool IsLocalOnly => true;
    public ObservableCollection<ContestCompetitorViewModel> Competitors { get; }
    public string Status { get => _status; private set => Set(ref _status, value); }
    public string RoundText => MaximumRounds == 0
        ? "ROUND —"
        : $"ROUND {Round:N0} · {MinimumRounds:N0}–{MaximumRounds:N0}";

    public event PropertyChangedEventHandler? PropertyChanged;

    public void AwardManualWin(string competitorId) => Award(competitorId, ContestAdjudicationKind.ManualWin);

    public void AwardInstantWin(string competitorId) => Award(competitorId, ContestAdjudicationKind.InstantWin);

    private void Award(string competitorId, ContestAdjudicationKind kind)
    {
        var competitor = Competitors.SingleOrDefault(value => value.Id == competitorId);
        if (competitor is null)
        {
            Status = "COMPETITOR NOT FOUND";
            return;
        }
        if (!competitor.CanAdjudicate)
        {
            Status = $"{competitor.Name.ToUpperInvariant()} CANNOT BE ADJUDICATED";
            return;
        }

        competitor.Record(kind);
        _submit(new ContestAdjudication(kind, ContestName, Round, competitor.Id));
        Status = $"{(kind == ContestAdjudicationKind.InstantWin ? "INSTANT WIN" : "MANUAL WIN")} · {competitor.Name.ToUpperInvariant()}";
    }

    private void Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}

public sealed class LocalContestAdjudicationLog
{
    public List<ContestAdjudication> Entries { get; } = [];
    public void Add(ContestAdjudication adjudication) => Entries.Add(adjudication);
}
