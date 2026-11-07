package com.snapious;

import com.snapious.ui.MainViewController;
import javax.swing.*;
import java.awt.*;

/**
 * Main application - Snapious BioTools suite.
 * Complete bioinformatics suite with sequence analysis, alignment,
 * phylogeny and visual display.
 */
public class SnapiousApp {

    private static final String APP_TITLE = "Snapious BioTools - Complete Bioinformatics Suite";

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

            // Load main panel
            try {
                MainViewController controller = new MainViewController();
                frame.setContentPane(controller.getPanel());
            } catch (Exception e) {
                System.err.println("Error loading the user interface: " + e.getMessage());
                e.printStackTrace();

                // Fallback: error panel
                JFrame errorFrame = new JFrame("Error");
                JTextArea errorText = new JTextArea(
                    "The user interface could not be loaded.\n\n" + e.getMessage()
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
