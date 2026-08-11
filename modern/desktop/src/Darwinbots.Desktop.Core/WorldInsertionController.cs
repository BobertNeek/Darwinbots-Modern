namespace Darwinbots.Desktop.Core;

public sealed class WorldInsertionController
{
    private readonly SimulationSession _session;

    public WorldInsertionController(SimulationSession session)
    {
        _session = session ?? throw new ArgumentNullException(nameof(session));
    }

    public SpeciesImport? SelectedSpecies { get; private set; }

    public bool IsActive { get; private set; }

    public void SelectSpecies(SpeciesImport? species)
    {
        if (species is null)
        {
            SelectedSpecies = null;
            IsActive = false;
            return;
        }

        species.Validate();
        SelectedSpecies = species;
    }

    public void Begin()
    {
        if (SelectedSpecies is null)
            throw new InvalidOperationException("Select a species before entering insertion mode.");
        IsActive = true;
    }

    public void Cancel() => IsActive = false;

    public Task<DnaImportReport> PlaceAsync(float[] position, CancellationToken cancellationToken = default)
    {
        if (!IsActive)
            throw new InvalidOperationException("Insertion mode is not active.");
        if (SelectedSpecies is not { } species)
            throw new InvalidOperationException("Select a species before placing an organism.");
        return _session.InsertSpeciesAtAsync(species, position, cancellationToken);
    }
}
