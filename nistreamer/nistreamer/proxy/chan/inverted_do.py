from nistreamer.channel import DOChanProxy


class InvertedDOChan(DOChanProxy):
    @property
    def default_val(self):
        return 'Off' if super().default_val else 'On'

    def on(self, t, dur):
        return super().low(t=t, dur=dur)

    def off(self, t, dur):
        return super().high(t=t, dur=dur)

    def go_on(self, t):
        return super().go_low(t=t)

    def go_off(self, t):
        return super().go_high(t=t)
