# Plot axis ranges

Available from **0.2.14**.

Right-click a two-dimensional plot and choose **Axis range…**. Enter a minimum
and maximum for each axis in its displayed units. Leave any field blank, or
type **Auto**, to retain an automatic endpoint. For example, set **Y minimum**
to `0` and leave **Y maximum** blank to keep a zero baseline as new data arrive.
Choose **Apply** to change the view, or **Cancel** to discard the edit.

**All Auto** clears the four fields; apply it to restore the plot's natural
range. **Reset axes to Auto** in the context menu restores it immediately.
Automatic bounds include the plot's usual padding and physical-domain defaults.
If a manual endpoint excludes all current data, the automatic endpoint expands
enough to keep a nonzero display range. Both numeric endpoints must be finite,
with the minimum below the maximum. Logarithmic axes require positive bounds.
Invalid entries leave the previous range in use and show an explanation.

Live and Series fit plots retain their limits while frames update. Different
fit parameters and derived expressions have separate ranges. The main Live
plot and its compact monitor share limits for the same displayed quantity.
If a frame arrives while the editor is open, Apply targets the current plot.
Ranges are display preferences for this application session; they are not
stored in project files. Ordinary pan and zoom remain available within a view.

Axis limits do not crop source arrays, change processing or fitting windows,
rerun a fit, or change parameter uncertainties. Image exports use the current
view; CSV exports retain the full exported data. The implementation is in
[`plot_ranges.rs`](../crates/rexafs-gui/src/plot_ranges.rs). Regression tests
cover independent automatic endpoints, changing data, invalid ranges, logarithmic
domains, reversed axes and an exact zero baseline on a narrow parameter trend.
