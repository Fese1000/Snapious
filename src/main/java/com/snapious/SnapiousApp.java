package com.snapious;

import com.snapious.ui.MainViewController;
import javax.swing.*;
import java.awt.*;

/**
 * Hauptanwendung - Snapious BioTools Suite.
 * Komplette Bioinformatik-Suite mit Sequenzanalyse, Alignment,
 * Phylogenie und visueller Darstellung.
 */
public class SnapiousApp {

    private static final String APP_TITLE = "Snapious BioTools - Komplette Bioinformatik-Suite";

    public static void main(String[] args) {
        SwingUtilities.invokeLater(() -> {
            try {
                UIManager.setLookAndFeel(UIManager.getSystemLookAndFeelClassName());
            } catch (Exception e) {
                e.printStackTrace();
            }

            JFrame frame = new JFrame(APP_TITLE);
            frame.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
            frame.setSize(1400, 900);

            // Hauptpanel laden
            try {
                MainViewController controller = new MainViewController();
                frame.setContentPane(controller.getPanel());
            } catch (Exception e) {
                System.err.println("Fehler beim Laden der Benutzeroberfläche: " + e.getMessage());
                e.printStackTrace();

                // Fallback: Error-Panel
                JFrame errorFrame = new JFrame("Fehler");
                JTextArea errorText = new JTextArea(
                    "Die Benutzeroberfläche konnte nicht geladen werden.\n\n" + e.getMessage()
                );
                errorText.setEditable(false);
                errorFrame.add(errorText);
                errorFrame.setSize(600, 400);
                errorFrame.setLocationRelativeTo(null);
                errorFrame.setVisible(true);
                return;
            }

            frame.setLocationRelativeTo(null);
            frame.setVisible(true);
        });
    }
}
