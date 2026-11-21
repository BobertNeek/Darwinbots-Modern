using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using Darwinbots.Desktop.Core;
using System.Text;

namespace Darwinbots.Desktop.Views;

public sealed class RobotConsoleWindow : Window
{
    private readonly RobotConsoleController _controller;
    private readonly OrganismKey _focus;
    private readonly ulong _uniqueId;
    private readonly TextBox _transcript;
    private readonly TextBox _command;
    private bool _commandInFlight;

    public RobotConsoleWindow(
        SimulationSession session,
        OrganismKey focus,
        Func<Task>? play = null,
        Func<Task>? pause = null,
        Func<uint, CancellationToken, Task<(ulong Tick, ulong Advanced)>>? advance = null,
        ulong uniqueId = 0)
    {
        Title = $"Darwinbots — Robot Console {focus.Slot}:{focus.Generation}";
        Width = 760;
        Height = 620;
        MinWidth = 560;
        MinHeight = 420;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");
        _controller = new RobotConsoleController(session, focus, play, pause, advance);
        _focus = focus;
        _uniqueId = uniqueId;

        var root = new Grid { RowDefinitions = new RowDefinitions("66,*,86") };
        root.Children.Add(new Border
        {
            Background = Brush.Parse("#FBF9F2"),
            BorderBrush = Brush.Parse("#C8C3B6"),
            BorderThickness = new Thickness(0, 0, 0, 1),
            Padding = new Thickness(20, 12),
            Child = new StackPanel
            {
                Spacing = 2,
                Children =
                {
                    new TextBlock { Text = "ROBOT CONSOLE", FontFamily = new FontFamily("Bahnschrift"), FontSize = 21, FontWeight = FontWeight.Bold, LetterSpacing = 1.3 },
                    new TextBlock { Text = "COMMANDS RUN AGAINST THE FOCUSED RUST SESSION", Foreground = Brush.Parse("#537B43"), FontFamily = new FontFamily("Bahnschrift"), FontSize = 10, FontWeight = FontWeight.Bold, LetterSpacing = 1.0 },
                },
            },
        });

        _transcript = new TextBox
        {
            IsReadOnly = true,
            AcceptsReturn = true,
            TextWrapping = TextWrapping.Wrap,
            Background = Brush.Parse("#FFFCF5"),
            Foreground = Brush.Parse("#1D2824"),
            BorderBrush = Brush.Parse("#D4CFC1"),
            BorderThickness = new Thickness(1),
            Padding = new Thickness(16),
            Margin = new Thickness(16, 14, 16, 8),
            FontFamily = new FontFamily("Cascadia Mono"),
            FontSize = 13,
            Text = "Darwinbots robot console\nType help for commands.\n",
        };
        Grid.SetRow(_transcript, 1);
        root.Children.Add(_transcript);

        _command = new TextBox
        {
            PlaceholderText = "command, for example: cycle 1 or set 50 100",
            FontFamily = new FontFamily("Cascadia Mono"),
            FontSize = 13,
            MinWidth = 280,
        };
        _command.KeyDown += Command_KeyDown;
        var run = Button("RUN", RunCommand_Click);
        var step = Button("STEP", (_, _) => _ = RunCommandAsync("step"));
        var eyes = Button("EYES", (_, _) => _ = RunCommandAsync("eyes"));
        var debug = Button("DEBUG", (_, _) => _ = RunCommandAsync("debug"));
        var dna = Button("DNA", (_, _) => _ = RunCommandAsync("showdna"));
        var playButton = Button("PLAY", async (_, _) => await RunCommandAsync("play"));
        var pauseButton = Button("PAUSE", async (_, _) => await RunCommandAsync("pause"));
        var controls = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 6,
            Children = { _command, run, step, eyes, debug, dna, playButton, pauseButton },
        };
        var footer = new Border
        {
            Background = Brush.Parse("#E5E0D4"),
            BorderBrush = Brush.Parse("#C8C3B6"),
            BorderThickness = new Thickness(0, 1, 0, 0),
            Padding = new Thickness(12, 14),
            Child = controls,
        };
        Grid.SetRow(footer, 2);
        root.Children.Add(footer);
        Content = root;
        Opened += (_, _) => _command.Focus();
    }

    private Button Button(string text, EventHandler<RoutedEventArgs> handler)
    {
        var button = new Button
        {
            Content = text,
            Background = Brush.Parse("#416E35"),
            Foreground = Brushes.White,
            FontWeight = FontWeight.Bold,
            Padding = new Thickness(10, 6),
        };
        button.Click += handler;
        return button;
    }

    internal bool RecordNativeCycle(EngineSnapshot snapshot, ulong previousTick)
    {
        var organism = snapshot.Organisms.FirstOrDefault(actor =>
            actor.Slot == _focus.Slot && actor.Generation == _focus.Generation
            && (_uniqueId == 0 || actor.UniqueId == _uniqueId));
        if (organism is not null)
        {
            // The DNA phase records the incremented internal tick. Earlier execrob
            // commands and skipped DNA phases cannot supply this cycle's frame.
            if (snapshot.Tick > previousTick
                && organism.ActivationHistory.LastOrDefault(frame => frame.Tick == snapshot.Tick) is { } frame)
            {
                var output = new StringBuilder();
                output.AppendLine().AppendLine("***ROBOT GENES EXECUTION***");
                for (var gene = 1; gene < frame.Activations.Length; gene++)
                    output.Append(gene).AppendLine(frame.Activations[gene] ? " executed" : " not executed");
                AppendNativeTranscript(output.ToString());
            }
            return true;
        }
        AppendNativeTranscript("Robot has died.\n");
        return false;
    }

    private void AppendNativeTranscript(string output)
    {
        var text = _transcript.Text + output;
        const int transcriptLimit = 2_500;
        if (text.Length > transcriptLimit)
        {
            var boundary = text.IndexOf('\n', text.Length - transcriptLimit);
            if (boundary >= 0) text = text[(boundary + 1)..];
        }
        _transcript.Text = text;
        _transcript.CaretIndex = text.Length;
    }

    private void Command_KeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key != Key.Enter) return;
        e.Handled = true;
        _ = RunCommandAsync(_command.Text ?? string.Empty);
    }

    private void RunCommand_Click(object? sender, RoutedEventArgs e) =>
        _ = RunCommandAsync(_command.Text ?? string.Empty);

    private async Task RunCommandAsync(string input)
    {
        if (_commandInFlight || string.IsNullOrWhiteSpace(input)) return;
        _commandInFlight = true;
        try
        {
            _transcript.Text += $"> {input.Trim()}\n";
            _command.Clear();
            var result = await _controller.ExecuteAsync(input).ConfigureAwait(true);
            _transcript.Text += result + "\n";
            _transcript.CaretIndex = _transcript.Text?.Length ?? 0;
        }
        catch (Exception error)
        {
            _transcript.Text += $"ERROR: {error.Message}\n";
        }
        finally { _commandInFlight = false; }
    }
}
